package e2e

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"testing"
	"time"
)

// f19 is a world whose server holds a vault key, a stub Linear, and one run of agent-one that uses it through the pod.
type f19 struct {
	*world
	linear *stubLinear
	r      *run
	conn   string
	seen   map[string][]byte // every agent-visible payload so far, by what it was
	parked string            // the decision of the approved write
	args   map[string]any    // the approved write's arguments
	// the decisions of the rejected and the expired writes
	rejected, expired string
}

var f19VaultKey = base64.StdEncoding.EncodeToString([]byte(strings.Repeat("v", 32)))

type connReceipt struct {
	ID, PodID, ConnectionID, Operation, ArgsHash, RunID, AgentID, Outcome, DecisionID, CreatedAt, SettledAt string
	UpstreamStatus                                                                                      int
	Approver                                                                                            *Actor
	Result                                                                                              map[string]any
}

// raw sends a JSON request and returns the status and the body as sent, for the token scans.
func (f *f19) raw(method, path, tok string, in any) (int, []byte) {
	var body io.Reader
	if in != nil {
		b, _ := json.Marshal(in)
		body = bytes.NewReader(b)
	}
	req, _ := http.NewRequest(method, f.srv.base()+path, body)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return -1, []byte(err.Error())
	}
	defer resp.Body.Close()
	b, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, b
}

// tool calls a pod operation over MCP with the run token, as the agent does. It returns the operation's own JSON answer.
func (f *f19) tool(name string, args map[string]any) (map[string]any, bool, []byte) {
	call := map[string]any{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": map[string]any{"name": name, "arguments": args}}
	_, b := f.raw("POST", "/mcp", f.r.tok, call)
	var env struct {
		Result struct {
			StructuredContent map[string]any
			IsError           bool
		}
	}
	_ = json.Unmarshal(b, &env)
	f.stamp(f.r.id, f.now())
	return env.Result.StructuredContent, env.Result.IsError, b
}

func (f *f19) connector(op string, args map[string]any) (map[string]any, bool, []byte) {
	return f.tool("jm_connector_call", map[string]any{"connectionId": f.conn, "operation": op, "args": args})
}

func (f *f19) keep(what string, b []byte) { f.seen[what] = b }

func (f *f19) grant(access string) error {
	st, b := f.raw("POST", f.podPath("grants"), f.owner.tok, map[string]any{"connectionId": f.conn, "agentId": f.a1.id, "access": access})
	if st != 200 {
		return fmt.Errorf("grant %s: HTTP %d %s", access, st, b)
	}
	return nil
}

// Step 1: only the pod owner connects; a drifted upstream is refused; the grant names a pod agent.
func (f *f19) connects() error {
	in := func(url string) map[string]any { return map[string]any{"vendor": "linear", "url": url, "token": stubToken} }
	if st, b := f.raw("POST", f.podPath("connections"), f.ben.tok, in(f.linear.srv.URL)); st != 403 || !strings.Contains(string(b), `"forbidden"`) {
		return fmt.Errorf("a member's connect: HTTP %d %s, want 403 forbidden", st, b)
	}
	drifted := newStubLinear(f.t, "create_issue")
	if st, b := f.raw("POST", f.podPath("connections"), f.owner.tok, in(drifted.srv.URL)); st != 409 || !strings.Contains(string(b), `"connector_drift"`) || !strings.Contains(string(b), "create_issue") {
		return fmt.Errorf("a connect to an upstream missing a pinned operation: HTTP %d %s, want 409 connector_drift naming create_issue", st, b)
	}
	st, b := f.raw("POST", f.podPath("connections"), f.owner.tok, in(f.linear.srv.URL))
	var c struct {
		ID, Vendor string
		Operations []struct{ Name, Class string }
	}
	if st != 200 || json.Unmarshal(b, &c) != nil || !strings.HasPrefix(c.ID, "con_") || len(c.Operations) != 3 {
		return fmt.Errorf("the owner's connect: HTTP %d %s, want a con_ connection with the 3 pinned operations", st, b)
	}
	if strings.Contains(string(b), stubToken) {
		return fmt.Errorf("the connect answer carries the token: %s", b)
	}
	f.conn = c.ID
	if st, b := f.raw("POST", f.podPath("grants"), f.owner.tok, map[string]any{"connectionId": f.conn, "agentId": "agt_00000000000000000000000000", "access": "read"}); st != 404 {
		return fmt.Errorf("a grant to an agent not in the pod: HTTP %d %s, want 404", st, b)
	}
	if st, _ := f.raw("POST", f.podPath("grants"), f.owner.tok, map[string]any{"connectionId": f.conn, "agentId": f.a1.id}); st != 400 {
		return fmt.Errorf("a grant with no access: HTTP %d, want 400", st)
	}
	out, isErr, b := f.connector("list_issues", map[string]any{})
	f.keep("the read before any grant", b)
	if !isErr || !strings.Contains(string(b), "forbidden") || out["state"] != nil || len(f.linear.seen("list_issues")) != 0 {
		return fmt.Errorf("a read before any grant: %s, want forbidden and no upstream call", b)
	}
	return f.grant("read")
}

// Step 2: the agent reads through the grant; the stub saw exactly one call, with the pod's bearer.
func (f *f19) reads() error {
	out, isErr, b := f.connector("list_issues", map[string]any{"query": "login"})
	f.keep("the read result", b)
	result, _ := out["result"].(map[string]any)
	issues, _ := result["issues"].([]any)
	if isErr || out["state"] != "done" || len(issues) != 1 || !strings.Contains(string(b), "ISS-1") {
		return fmt.Errorf("connector_call list_issues: %s, want state done with the stub's one issue", b)
	}
	calls := f.linear.seen("list_issues")
	if len(calls) != 1 || calls[0].Bearer != "Bearer "+stubToken || calls[0].Args["query"] != "login" {
		return fmt.Errorf("the stub saw %d list_issues calls (%+v), want exactly one with the stub bearer and query login", len(calls), calls)
	}
	return nil
}

// refusals collects the errors an agent can get, including an upstream error that echoes the credential.
func (f *f19) refusals() error {
	cases := []struct {
		what, op, code string
		args           map[string]any
	}{
		{"an upstream error that echoes the token", "get_issue", "upstream_failed", map[string]any{"id": "echo"}},
		{"an upstream HTTP error that echoes the token \\u-escaped in JSON", "get_issue", "upstream_failed", map[string]any{"id": "echo-u"}},
		{"an upstream HTTP error that echoes the token URL-encoded", "get_issue", "upstream_failed", map[string]any{"id": "echo-url"}},
		{"an upstream HTTP error that echoes the bearer base64-encoded", "get_issue", "upstream_failed", map[string]any{"id": "echo-b64"}},
		{"an operation the pin does not name", "delete_issue", "not_found", map[string]any{"id": "ISS-1"}},
		{"a write on a read grant", "create_issue", "forbidden", map[string]any{"title": "x", "teamId": "T"}},
	}
	for _, c := range cases {
		_, isErr, b := f.connector(c.op, c.args)
		f.keep(c.what, b)
		if !isErr || !strings.Contains(string(b), c.code) {
			return fmt.Errorf("%s: %s, want an error with %s", c.what, b, c.code)
		}
	}
	if n := len(f.linear.seen("create_issue")) + len(f.linear.seen("delete_issue")); n != 0 {
		return fmt.Errorf("the stub got %d refused calls", n)
	}
	return nil
}

// envOfNewRun wakes agent-one again and claims the run as the daemon does: the claim answer is everything the agent's
// environment is built from (run token, brief, mcpUrl).
func (f *f19) envOfNewRun() []byte {
	mk := f.compA.sock.mark()
	f.newTask("read the board again", f.a1.id)
	fr, _ := f.compA.sock.waitAfter(mk, wakeFor(f.a1.id), 5*time.Second)
	var wk Wake
	fr.dec(&wk)
	_, b := f.raw("POST", "/v1/runs", f.compA.tok, map[string]any{"wakeId": wk.WakeID})
	return b
}

// Step 3: the token is in none of what the agent sees, nor in the server's log.
func (f *f19) tokenAbsent() error {
	if err := f.refusals(); err != nil {
		return err
	}
	_, b := f.raw("POST", "/mcp", f.r.tok, map[string]any{"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
	f.keep("the MCP tool list", b)
	out, _, b := f.tool("jm_connector_list", map[string]any{})
	f.keep("the agent's operation list", b)
	if s := string(b); !strings.Contains(s, "list_issues") || strings.Contains(s, "create_issue") || strings.Contains(s, "delete_issue") || out == nil {
		return fmt.Errorf("the agent's operation list on a read grant: %s, want the pinned reads only", b)
	}
	_, _, b = f.tool("jm_connector_receipts", map[string]any{})
	f.keep("the receipts", b)
	f.keep("the run claim answer (the agent's env)", f.envOfNewRun())
	f.keep("the server log", f.srv.log.bytes())
	return f.noLeak()
}

// noLeak checks every agent-visible payload kept so far for the token in any of its encoded forms.
func (f *f19) noLeak() error {
	for what, b := range f.seen {
		if form := leakIn(b); form != "" {
			return fmt.Errorf("the Linear token (%s) is in %s: %.300s", form, what, b)
		}
	}
	return nil
}

// TestF19: an agent reads Linear through a pod grant without ever holding the Linear token (feature_list.json F19).
func TestF19(t *testing.T) {
	needFake(t)
	par(t)
	f := &f19{world: newWorldAt(t, fakeBase, "JM_VAULT_KEY="+f19VaultKey), linear: newStubLinear(t, ""), seen: map[string][]byte{}}
	f.r = f.startRun(f.a1, "triage the login bug")
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.sO.snapshot(0) }}
	defer p.count(t)
	steps := []struct {
		name string
		fn   func() error
	}{
		{"against a stub Linear, a pod admin connects Linear and grants an agent read access through the pod", f.connects},
		{"the agent reads Linear through the pod grant, and the result comes back to it", f.reads},
		{"the Linear token is absent from every agent-visible payload: tool lists, results, errors and the run's env", f.tokenAbsent},
		{"a write call from the agent parks as a decision card with the exact arguments, and the stub Linear sees no write while it is parked", f.parks},
		{"a connection's approver set: a listed member approves, an unlisted member is refused, and a removed approver is refused", f.approverSet},
		{"one approval releases exactly one upstream call; a rejection or expiry releases none", f.releasesOnce},
		{"the receipt is stamped by the server and links the card, the approver and the result", f.receipt},
	}
	for _, s := range steps {
		p.step(s.name, s.fn)
	}
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}

// canonHash is the receipt's argsHash: SHA-256 of the arguments as JSON with sorted keys (Go sorts map keys).
func canonHash(args map[string]any) string {
	b, _ := json.Marshal(args)
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:])
}
