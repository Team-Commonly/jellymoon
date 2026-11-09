package e2e

import (
	"flag"
	"fmt"
	"testing"
	"time"
)

// budgetVerdict judges a run: what the budget test does with the suite tests' outcomes and the time taken. verdict is
// "skip" (a filtered run, not the whole suite), "fail" or "" (within budget). filtered is true when -run narrowed the run.
func budgetVerdict(names []string, outcome func(name string) (ran, failed bool), took time.Duration, filtered bool) (verdict, msg string) {
	var missing, failed []string
	for _, name := range names {
		if ran, bad := outcome(name); !ran {
			missing = append(missing, name)
		} else if bad {
			failed = append(failed, name)
		}
	}
	if len(missing) > 0 {
		if filtered {
			return "skip", fmt.Sprintf("partial run (not run: %v); the budget needs the whole suite in one invocation", missing)
		}
		// A whole-suite run in which a suite test never reported means the budget was judged too early or the test was lost.
		return "fail", fmt.Sprintf("the whole suite was asked for and these suite tests never reported: %v", missing)
	}
	if len(failed) > 0 {
		return "fail", fmt.Sprintf("the suite did not pass, so there is no budget to meet: failed %v", failed)
	}
	if took >= 60*time.Second {
		return "fail", fmt.Sprintf("suite took %v, budget is 60 s", took)
	}
	return "", ""
}

// TestBudgetVerdict checks the budget test's own judgment with made-up outcomes; it needs no server.
func TestBudgetVerdict(t *testing.T) {
	names := []string{"A", "B"}
	of := func(m map[string]bool) func(string) (bool, bool) {
		return func(n string) (bool, bool) { v, ok := m[n]; return ok, v }
	}
	for _, c := range []struct {
		name     string
		out      map[string]bool
		took     time.Duration
		filtered bool
		want     string
	}{
		{"all passed in time", map[string]bool{"A": false, "B": false}, 30 * time.Second, false, ""},
		{"a test is missing in a whole run", map[string]bool{"A": false}, time.Second, false, "fail"},
		{"a test is missing in a filtered run", map[string]bool{"A": false}, time.Second, true, "skip"},
		{"a test failed", map[string]bool{"A": false, "B": true}, time.Second, false, "fail"},
		{"too slow", map[string]bool{"A": false, "B": false}, 60 * time.Second, false, "fail"},
	} {
		if got, msg := budgetVerdict(names, of(c.out), c.took, c.filtered); got != c.want {
			t.Errorf("%s: verdict %q (%s), want %q", c.name, got, msg, c.want)
		}
	}
}

// TestSuiteParallel checks the slot count TestMain gives the run; it needs no server.
func TestSuiteParallel(t *testing.T) {
	for _, c := range []struct {
		name         string
		given, procs int
		set          bool
		want         int
	}{
		{"unset: one slot per CPU plus the budget test's", 4, 4, false, 5},
		{"unset on one CPU: two, so the budget test cannot deadlock", 1, 1, false, 2},
		{"set by hand: kept as given", 3, 4, true, 3},
		{"set to 1 by hand: kept, and the budget test says so", 1, 4, true, 1},
	} {
		if got := suiteParallel(c.set, c.given, c.procs); got != c.want {
			t.Errorf("%s: %d, want %d", c.name, got, c.want)
		}
	}
}

// TestF14Budget: the whole suite, run in one invocation against a fake clock, finishes in under 60 s.
// The suite tests run in parallel (par), so this one is parallel too and waits for all of them to finish before it
// judges the run. It fails if a suite test failed or never reported, and skips on a run narrowed with -run.
func TestF14Budget(t *testing.T) {
	needFake(t)
	if p := flag.Lookup("test.parallel"); p != nil && p.Value.String() == "1" {
		t.Fatal("the suite runs its tests in parallel and the budget test waits for them; -parallel 1 would deadlock it, use 2 or more")
	}
	t.Parallel()
	suiteWG.Wait()
	countTest(t)
	outcome := func(name string) (bool, bool) {
		v, ok := outcomes.Load(name)
		return ok, ok && v.(bool)
	}
	filtered := false
	if f := flag.Lookup("test.run"); f != nil && f.Value.String() != "" {
		filtered = true
	}
	switch verdict, msg := budgetVerdict(suiteTests, outcome, time.Since(suiteStart), filtered); verdict {
	case "skip":
		t.Skip(msg)
	case "fail":
		t.Fatal(msg)
	}
}
