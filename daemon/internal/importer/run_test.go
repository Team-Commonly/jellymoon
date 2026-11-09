package importer

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/e2e/cmyfake"
)

// The end-to-end scenarios (dry run, skips, refusals, a planted secret, idempotence, kill and resume, backoff, opt-ins) live in
// e2e/imp_test.go. These tests are the importer's own logic, run against the same fixture Commonly.

type obj = cmyfake.Obj

func runWith(t *testing.T, f *cmyfake.Server, sink Sink, o Options) (*Report, error) {
	t.Helper()
	c, err := NewCommonly(f.Serve(t).URL, "cm_sentinelTOKEN0001")
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

// A post that lands but whose answer is lost (a kill between the answer and the state write) is found again by its marker.
func TestALostAcknowledgementDoesNotDuplicate(t *testing.T) {
	f, sink, o := cmyfake.New(), newSink(), opts(t, true)
	sink.lostAck = 2
	if _, err := runWith(t, f, sink, o); err == nil {
		t.Fatal("the first run should stop")
	}
	if _, err := runWith(t, f, sink, o); err != nil {
		t.Fatal(err)
	}
	seen := map[string]int{}
	for _, b := range sink.bodies() {
		seen[markerOf(b)]++
	}
	for _, id := range []string{"m1", "m2", "m3", "s1", "s2"} {
		if seen[sourceMarker(id)] != 1 {
			t.Errorf("%s posted %d times (%v)", id, seen[sourceMarker(id)], seen)
		}
	}
}

// Messages tied on one timestamp across a page boundary are all read.
func TestTiedTimestampsAcrossAPageBoundaryAreAllRead(t *testing.T) {
	f := cmyfake.New()
	var ms []obj
	for i := 0; i < 52; i++ {
		sec := i
		if i < 3 {
			sec = 0
		}
		ms = append(ms, cmyfake.Msg(fmt.Sprintf("t%02d", i), "u_bob", "Bob Builder", "msg", fmt.Sprintf("2026-02-01T00:00:%02d.000Z", sec)))
	}
	f.Msgs["p_study"] = ms
	rep, err := runWith(t, f, newSink(), opts(t, false))
	if err != nil || rep.Messages != 52+3 {
		t.Fatalf("%v, messages %d", err, rep.Messages)
	}
}

// The body prefix and the attachment placeholder: a name, never the file or its url.
func TestBodyCarriesThePrefixAndAttachmentPlaceholder(t *testing.T) {
	var m Message
	raw := `{"id":"m1","user_id":"u1","username":"Bob Builder","content":"see plan","messageType":"text","createdAt":"2026-01-01T10:00:00Z",` +
		`"payload":{"attachments":[{"originalName":"plan.pdf","url":"https://store.example/private/abc?sig=1"}]}}`
	if err := json.Unmarshal([]byte(raw), &m); err != nil {
		t.Fatal(err)
	}
	n := m.Normalize()
	body := *compose(n, n.Author).Body
	if !strings.Contains(body, "[imported · Bob Builder · 2026-01-01T10:00:00.000Z]") || !strings.Contains(body, "[attachment: plan.pdf]") || strings.Contains(body, "store.example") {
		t.Errorf("body:\n%s", body)
	}
}

// The state file holds ids only: with --anonymize and a message that has no author id, no display name reaches it.
func TestStateHoldsIDsNotNames(t *testing.T) {
	f, o := cmyfake.New(), opts(t, true)
	o.Anonymize = true
	delete(f.Msgs["p_study"][0], "user_id")
	if _, err := runWith(t, f, newSink(), o); err != nil {
		t.Fatal(err)
	}
	state, _ := os.ReadFile(o.StatePath)
	if strings.Contains(string(state), "Bob") || strings.Contains(string(state), "Myself") || strings.Contains(string(state), "example.com") {
		t.Errorf("the state file holds a name: %s", state)
	}
}

// Agent cards take the agent's own name and instance from the pod listing: GET /api/pods/:id does not populate botMetadata.
func TestAgentCardsComeFromTheListing(t *testing.T) {
	f, o := cmyfake.New(), opts(t, true)
	if _, err := runWith(t, f, newSink(), o); err != nil {
		t.Fatal(err)
	}
	raw, _ := os.ReadFile(filepath.Join(o.OutDir, "agent-cards.json"))
	var cards []Card
	if err := json.Unmarshal(raw, &cards); err != nil || len(cards) != 1 {
		t.Fatalf("%v: %s", err, raw)
	}
	if cards[0].DisplayName != "Helper Bot (agent)" || cards[0].Instance != "inst-a_bot" || cards[0].Handle != "Helper Bot" {
		t.Errorf("card %+v", cards[0])
	}
}

// A 401 or 403 on one pod's messages skips that pod and counts it; the other pods still import.
func TestAnUnauthorizedPodIsSkippedAndCounted(t *testing.T) {
	for _, code := range []int{401, 403} {
		f, sink := cmyfake.New(), newSink()
		f.FailMsg = func(pod string, call int) (int, string) {
			if pod == "p_study" {
				return code, ""
			}
			return 0, ""
		}
		rep, err := runWith(t, f, sink, opts(t, true))
		if err != nil || rep.PodsImported != 1 || rep.PodsSkipped["unauthorized"] != 1 || rep.MessagesPosted != 3 {
			t.Errorf("%d: %v %+v", code, err, rep)
		}
	}
}

// A 401 on a LATER page drops the whole pod too: the rows of the pages already read are discarded, not half imported.
func TestAnUnauthorizedLaterPageSkipsTheWholePod(t *testing.T) {
	f, pages := cmyfake.New(), 0
	for i := 0; i < 60; i++ {
		f.Msgs["p_study"] = append(f.Msgs["p_study"], cmyfake.Msg(fmt.Sprintf("t%02d", i), "u_bob", "Bob", "msg", fmt.Sprintf("2026-02-01T00:00:%02d.000Z", i)))
	}
	f.FailMsg = func(pod string, _ int) (int, string) {
		if pod == "p_study" {
			if pages++; pages >= 2 {
				return 401, ""
			}
		}
		return 0, ""
	}
	rep, err := runWith(t, f, newSink(), opts(t, true))
	if err != nil || pages < 2 || rep.PodsImported != 1 || rep.PodsSkipped["unauthorized"] != 1 || rep.MessagesPosted != 3 {
		t.Errorf("pages %d: %v %+v", pages, err, rep)
	}
}

// A secret in a pod name goes through the same where() as every other scrub error, so a secret-shaped pod id is not echoed.
func TestPodNameSecretErrorDoesNotEchoASecretShapedPodID(t *testing.T) {
	f, secret := cmyfake.New(), "cm_"+"0123456789"+"abcdef0123456789"
	f.Pods[4]["_id"], f.Pods[4]["name"], f.Msgs[secret] = secret, "Study "+secret, f.Msgs["p_study"]
	if _, err := runWith(t, f, newSink(), opts(t, true)); !errors.Is(err, ErrSecret) || !strings.Contains(err.Error(), "a pod name") || strings.Contains(err.Error(), "0123456789") {
		t.Fatalf("err %v", err)
	}
}

// A secret fails the run and the error names where to look (pod id, message id), never the match; an id that is itself the secret is not echoed.
func TestSecretErrorNamesThePodAndMessage(t *testing.T) {
	f := cmyfake.New()
	f.Msgs["p_study"][1]["content"] = "my key is cm_0123456789abcdef0123456789"
	_, err := runWith(t, f, newSink(), opts(t, true))
	if !errors.Is(err, ErrSecret) || !strings.Contains(err.Error(), "p_study") || !strings.Contains(err.Error(), "s2") || strings.Contains(err.Error(), "0123456789abcdef") {
		t.Fatalf("err %v", err)
	}
	f = cmyfake.New()
	f.Msgs["p_study"][1]["id"] = "cm_0123456789abcdef0123456789"
	_, err = runWith(t, f, newSink(), opts(t, true))
	if !errors.Is(err, ErrSecret) || strings.Contains(err.Error(), "0123456789abcdef") {
		t.Fatalf("err %v", err)
	}
}

// A failed read names the route shape and the pod through where(), never the raw path: a GitHub- or cm_-shaped pod id (a clean name, a 404 or 400) is not echoed.
func TestFailedReadsNeverEchoASecretShapedPodID(t *testing.T) {
	for _, secret := range []string{"ghp_" + "0123456789" + "ABCDEFGHIJabcdefghij", "cm_" + "0123456789" + "abcdef0123456789"} {
		for _, code := range []int{404, 400} {
			f := cmyfake.New()
			f.Pods[4]["_id"], f.Msgs[secret] = secret, f.Msgs["p_study"]
			f.FailMsg = func(pod string, _ int) (int, string) {
				if pod == secret {
					return code, ""
				}
				return 0, ""
			}
			_, err := runWith(t, f, newSink(), opts(t, true))
			if err == nil || strings.Contains(err.Error(), "0123456789") || !strings.Contains(err.Error(), "/api/messages/:podId") {
				t.Errorf("%.4s %d: %v", secret, code, err)
			}
		}
	}
}

// Grants and integrations are filtered to the imported pods, unrevoked, and counted by what Jellymoon supports.
func TestGrantsAreFilteredAndCounted(t *testing.T) {
	f := cmyfake.New()
	f.Grants = []obj{
		{"connector": obj{"id": "linear"}, "writeMode": "read", "target": obj{"kind": "pod", "id": "p_team"}},
		{"connector": obj{"id": "github"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_study"}},
		{"connector": obj{"id": "linear"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_admin"}},
		{"connector": obj{"id": "linear"}, "writeMode": "write", "target": obj{"kind": "pod", "id": "p_team"}, "revokedAt": "2026-01-01T00:00:00Z"},
	}
	f.Integ["p_team"] = []obj{{"type": "discord", "status": "connected", "config": obj{"botToken": "SECRETBOTVALUE"}}}
	rep, err := runWith(t, f, newSink(), opts(t, false))
	if err != nil || rep.Grants != 2 || rep.GrantsUnsupported != 1 || rep.Integrations != 1 {
		t.Errorf("%v %+v", err, rep)
	}
}

func TestOutputsAreOwnerOnly(t *testing.T) {
	o := opts(t, true)
	if _, err := runWith(t, cmyfake.New(), newSink(), o); err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{o.StatePath, filepath.Join(o.OutDir, "agent-cards.json"), filepath.Join(o.OutDir, "grants-checklist.md")} {
		fi, err := os.Stat(p)
		if err != nil || fi.Mode().Perm()&0o077 != 0 {
			t.Errorf("%s: %v %v", p, err, fi)
		}
	}
}
