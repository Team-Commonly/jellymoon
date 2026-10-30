package e2e

import (
	"testing"
	"time"
)

// TestF14Budget: the whole suite, run in one invocation against a fake clock, finishes in under 60 s.
// It judges the run so far, so it is last (file order). It fails if a suite test failed and skips on a
// partial run.
func TestF14Budget(t *testing.T) {
	needFake(t)
	var missing, failed []string
	for _, name := range suiteTests {
		if v, ok := outcomes.Load(name); !ok {
			missing = append(missing, name)
		} else if v.(bool) {
			failed = append(failed, name)
		}
	}
	if len(missing) > 0 {
		t.Skipf("partial run (not run: %v); the budget needs the whole suite in one invocation", missing)
	}
	if len(failed) > 0 {
		t.Fatalf("the suite did not pass, so there is no budget to meet: failed %v", failed)
	}
	if took := time.Since(suiteStart); took >= 60*time.Second {
		t.Fatalf("suite took %v, budget is 60 s", took)
	}
}
