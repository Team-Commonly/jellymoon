package importer

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func runWith(t *testing.T, f *fx, sink Sink, o Options) (*Report, error) {
	t.Helper()
	c, err := NewCommonly(f.server().URL, "cm_sentinelTOKEN0001")
	if err != nil {
		t.Fatal(err)
	}
	c.sleep, c.gap = func(time.Duration) {}, 0
	return Run(c, sink, NewScrubber("cm_sentinelTOKEN0001"), o)
}

func opts(t *testing.T, apply bool) Options {
	d := t.TempDir()
	return Options{Apply: apply, StatePath: filepath.Join(d, "state.json"), OutDir: filepath.Join(d, "out"), Server: "http://jm.test"}
}

func TestDryRunWritesNothingAndCountsEverything(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, false)
	rep, err := runWith(t, f, sink, o)
	if err != nil {
		t.Fatal(err)
	}
	if sink.writes != 0 {
		t.Fatalf("dry run wrote %d times", sink.writes)
	}
	for _, p := range []string{o.StatePath, o.OutDir} {
		if _, err := os.Stat(p); err == nil {
			t.Errorf("dry run created %s", p)
		}
	}
	if rep.PodsImported != 2 || rep.Messages != 5 || rep.Cards != 1 {
		t.Errorf("report %+v", rep)
	}
	want := map[string]int{"dm-opt-in": 1, "agent-admin": 1, "not-a-member": 1}
	for k, n := range want {
		if rep.PodsSkipped[k] != n {
			t.Errorf("skipped %v, want %v", rep.PodsSkipped, want)
		}
	}
}

// The DM, the admin room and the pod the person is not in are never read, and nothing the importer must not call is called.
func TestNeverReadsWhatItSkipsOrCallsForbiddenRoutes(t *testing.T) {
	f := newFx(t)
	if _, err := runWith(t, f, newSink(), opts(t, false)); err != nil {
		t.Fatal(err)
	}
	for _, id := range []string{"p_dm", "p_admin", "p_foreign"} {
		if n := f.count("/api/messages/" + id); n != 0 {
			t.Errorf("%s was read %d times", id, n)
		}
	}
	for _, bad := range []string{"scope", "/api/users/", "/api/credentials", "/api/agents/runtime", "ingest-tokens", "/api/uploads"} {
		if n := f.count(bad); n != 0 {
			t.Errorf("%d requests carried %q", n, bad)
		}
	}
	for _, r := range f.reqs {
		if !strings.HasPrefix(r, "GET ") {
			t.Errorf("not a GET: %s", r)
		}
	}
}

func TestRefusesAnAdminOrABotBeforeAnyPodRead(t *testing.T) {
	for name, set := range map[string]func(*fx){
		"admin": func(f *fx) { f.me["role"] = "admin" },
		"bot":   func(f *fx) { f.me["isBot"] = true },
	} {
		f, sink := newFx(t), newSink()
		set(f)
		_, err := runWith(t, f, sink, opts(t, true))
		if !errors.Is(err, ErrRefused) {
			t.Errorf("%s: err %v", name, err)
		}
		if len(f.reqs) != 1 || f.count("/api/pods") != 0 || sink.writes != 0 {
			t.Errorf("%s: requests %v, writes %d", name, f.reqs, sink.writes)
		}
	}
}

func TestApplyTwicePostsNothingTheSecondTime(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	if rep, err := runWith(t, f, sink, o); err != nil || rep.MessagesPosted != 5 {
		t.Fatalf("first run: %v %+v", err, rep)
	}
	before := sink.writes
	rep, err := runWith(t, f, sink, o)
	if err != nil || rep.MessagesPosted != 0 || sink.writes != before {
		t.Fatalf("second run: %v %+v, writes %d -> %d", err, rep, before, sink.writes)
	}
	if len(sink.pods) != 2 {
		t.Errorf("pods %v", sink.pods)
	}
}

func TestIncludeDMOptsOneDMIn(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	o.IncludeDM = []string{"p_dm", "p_foreign"}
	rep, err := runWith(t, f, sink, o)
	if err != nil {
		t.Fatal(err)
	}
	if rep.PodsImported != 3 || f.count("/api/messages/p_dm") == 0 || f.count("/api/messages/p_foreign") != 0 || rep.PodsSkipped["not-a-member"] != 1 {
		t.Errorf("report %+v, requests %v", rep, f.reqs)
	}
}

func TestAnonymizeReplacesPeopleButNotAgents(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	o.Anonymize = true
	if _, err := runWith(t, f, sink, o); err != nil {
		t.Fatal(err)
	}
	all := strings.Join(sink.bodies(), "\n")
	for _, name := range []string{"Bob Builder", "Me Myself"} {
		if strings.Contains(all, name) {
			t.Errorf("%q survived anonymize", name)
		}
	}
	if !strings.Contains(all, "participant A") || !strings.Contains(all, "participant B") || !strings.Contains(all, "Helper Bot") {
		t.Errorf("bodies:\n%s", all)
	}
	state, _ := os.ReadFile(o.StatePath)
	if strings.Contains(string(state), "Bob") || strings.Contains(string(state), "example.test") {
		t.Errorf("state holds a name: %s", state)
	}
}

func TestBodyCarriesThePrefixAndAttachmentPlaceholder(t *testing.T) {
	f, sink := newFx(t), newSink()
	f.msgs["p_study"][0]["payload"] = obj{"attachments": []obj{{"originalName": "plan.pdf", "url": "https://store.example/private/abc?sig=1"}}}
	if _, err := runWith(t, f, sink, opts(t, true)); err != nil {
		t.Fatal(err)
	}
	all := strings.Join(sink.bodies(), "\n")
	if !strings.Contains(all, "[imported · Bob Builder · 2026-01-01T10:00:00.000Z]") || !strings.Contains(all, "[attachment: plan.pdf]") || strings.Contains(all, "store.example") {
		t.Errorf("bodies:\n%s", all)
	}
}

// A secret in a field the importer copies fails the whole run before the first write: nothing reaches Jellymoon, no state, no files.
func TestPlantedSecretFailsTheRunWithNothingWritten(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	f.msgs["p_study"][1]["content"] = "my key is cm_0123456789abcdef0123456789"
	_, err := runWith(t, f, sink, o)
	if !errors.Is(err, ErrSecret) {
		t.Fatalf("err %v", err)
	}
	if strings.Contains(err.Error(), "0123456789abcdef") {
		t.Errorf("the error repeats the secret: %v", err)
	}
	if sink.writes != 0 {
		t.Errorf("%d writes", sink.writes)
	}
	for _, p := range []string{o.StatePath, o.OutDir} {
		if _, e := os.Stat(p); e == nil {
			t.Errorf("%s exists", p)
		}
	}
}

func TestResumeAfterAFailureHasNoGapAndNoDuplicate(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	sink.failAfter = 3
	if _, err := runWith(t, f, sink, o); err == nil {
		t.Fatal("the first run should stop")
	}
	sink.failAfter = 0
	if _, err := runWith(t, f, sink, o); err != nil {
		t.Fatal(err)
	}
	assertEachOnce(t, sink, "m1", "m2", "m3", "s1", "s2")
}

// A post that lands but whose answer is lost (a kill between the answer and the state write) is found again by its marker.
func TestALostAcknowledgementDoesNotDuplicate(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	sink.lostAck = 2
	if _, err := runWith(t, f, sink, o); err == nil {
		t.Fatal("the first run should stop")
	}
	if _, err := runWith(t, f, sink, o); err != nil {
		t.Fatal(err)
	}
	assertEachOnce(t, sink, "m1", "m2", "m3", "s1", "s2")
}

func assertEachOnce(t *testing.T, sink *fakeSink, ids ...string) {
	t.Helper()
	seen := map[string]int{}
	for _, b := range sink.bodies() {
		seen[markerOf(b)]++
	}
	for _, id := range ids {
		if seen[sourceMarker(id)] != 1 {
			t.Errorf("%s posted %d times (%v)", id, seen[sourceMarker(id)], seen)
		}
	}
	if len(seen) != len(ids) {
		t.Errorf("posted %d distinct, want %d", len(seen), len(ids))
	}
}

// Messages tied on one timestamp across a page boundary are all read.
func TestTiedTimestampsAcrossAPageBoundaryAreAllRead(t *testing.T) {
	f := newFx(t)
	var ms []obj
	for i := 0; i < 52; i++ {
		sec := i
		if i < 3 {
			sec = 0
		}
		at := fmt.Sprintf("2026-02-01T00:00:%02d.000Z", sec)
		ms = append(ms, message(fmt.Sprintf("t%02d", i), "u_bob", "Bob Builder", "msg", at))
	}
	f.msgs["p_study"] = ms
	rep, err := runWith(t, f, newSink(), opts(t, false))
	if err != nil || rep.Messages != 52+3 {
		t.Fatalf("%v, messages %d", err, rep.Messages)
	}
}

// 429 then 503 on a pod's messages: the run retries, and never has two requests in flight (so never two pods).
func TestBacksOffAndNeverHasTwoRequestsInFlight(t *testing.T) {
	f := newFx(t)
	f.failMsg = func(podID string, call int) int {
		if podID == "p_team" && call == 1 {
			return 429
		}
		if podID == "p_team" && call == 2 {
			return 503
		}
		return 0
	}
	rep, err := runWith(t, f, newSink(), opts(t, false))
	if err != nil || rep.Messages != 5 {
		t.Fatalf("%v %+v", err, rep)
	}
	if f.maxCur.Load() != 1 {
		t.Errorf("up to %d requests in flight", f.maxCur.Load())
	}
}

func TestFiveFailuresStopTheRunAndKeepState(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	f.failMsg = func(podID string, call int) int {
		if podID == "p_study" {
			return 503
		}
		return 0
	}
	if _, err := runWith(t, f, sink, o); !errors.Is(err, ErrStopped) {
		t.Fatalf("err %v", err)
	}
	if sink.writes != 0 {
		t.Errorf("%d writes before the read finished", sink.writes)
	}
	f.failMsg = nil
	if _, err := runWith(t, f, sink, o); err != nil {
		t.Fatal(err)
	}
	assertEachOnce(t, sink, "m1", "m2", "m3", "s1", "s2")
}

// Grants and integrations come out through an allowlist: a token field, a bot token and a webhook url never reach a file.
func TestGrantsAndIntegrationsNeverLeakIntoTheChecklist(t *testing.T) {
	f, sink, o := newFx(t), newSink(), opts(t, true)
	f.grants = []obj{
		{"grantId": "g1", "connector": obj{"id": "linear", "label": "Linear"}, "writeMode": "read", "target": obj{"kind": "pod", "id": "p_team"}, "token": "SECRETGRANTVALUE", "accessToken": "SECRETGRANTVALUE"},
		{"grantId": "g2", "connector": obj{"id": "github", "label": "GitHub"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_study"}, "refreshToken": "SECRETGRANTVALUE"},
		{"grantId": "g3", "connector": obj{"id": "linear"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_admin"}},
		{"grantId": "g4", "connector": obj{"id": "linear"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_team"}, "revokedAt": "2026-01-01T00:00:00Z"},
	}
	f.integ["p_team"] = []obj{{"type": "discord", "status": "connected", "createdAt": "2026-01-01T00:00:00Z", "config": obj{"botToken": "SECRETBOTVALUE", "webhookUrl": "https://hooks.example/SECRETHOOKVALUE", "secretToken": "SECRETBOTVALUE"}}}
	rep, err := runWith(t, f, sink, o)
	if err != nil {
		t.Fatal(err)
	}
	if rep.Grants != 2 || rep.GrantsUnsupported != 1 || rep.Integrations != 1 {
		t.Errorf("report %+v", rep)
	}
	var all strings.Builder
	files, _ := os.ReadDir(o.OutDir)
	for _, e := range files {
		b, _ := os.ReadFile(filepath.Join(o.OutDir, e.Name()))
		all.Write(b)
	}
	all.WriteString(strings.Join(sink.bodies(), "\n"))
	for _, bad := range []string{"SECRET", "botToken", "webhookUrl", "bob@example.test", "me@example.test"} {
		if strings.Contains(all.String(), bad) {
			t.Errorf("output contains %q", bad)
		}
	}
	for _, want := range []string{"linear", "unsupported", "discord", "Helper Bot"} {
		if !strings.Contains(all.String(), want) {
			t.Errorf("output lacks %q:\n%s", want, all.String())
		}
	}
}

func TestOutputsAreOwnerOnly(t *testing.T) {
	o := opts(t, true)
	if _, err := runWith(t, newFx(t), newSink(), o); err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{o.StatePath, filepath.Join(o.OutDir, "agent-cards.json"), filepath.Join(o.OutDir, "grants-checklist.md")} {
		fi, err := os.Stat(p)
		if err != nil || fi.Mode().Perm()&0o077 != 0 {
			t.Errorf("%s: %v %v", p, err, fi)
		}
	}
}
