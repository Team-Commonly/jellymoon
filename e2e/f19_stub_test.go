package e2e

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"net/http/httptest"
	"slices"
	"sync"
	"testing"
)

// The stub Linear for TestF19: a tiny MCP-over-HTTP JSON-RPC server. It answers only a caller with the stub bearer, lists
// tools that match the pinned catalogue (plus one the pin leaves out), and records every tools/call it receives.
// It holds a slash and a plus, so its URL-encoded and base64 forms differ from it.
const stubToken = "stub-linear/credential+for-the-f19-suite-only"

type stubCall struct {
	Name, Bearer string
	Args         map[string]any
}

type stubLinear struct {
	srv   *httptest.Server
	mu    sync.Mutex
	calls []stubCall
	tools []map[string]any
}

// pinnedTools is what the stub lists: the three operations the repo's pin names, with the same argument names, and
// delete_issue, which the pin does not name (an upstream may list more than the pin; the agent must never see it).
func pinnedTools() []map[string]any {
	tool := func(name string, required []string, props ...string) map[string]any {
		p := map[string]any{}
		for _, k := range props {
			p[k] = map[string]any{"type": "string"}
		}
		// Every tool claims to be read-only, as an upstream annotation might; the pod must take the class from its pin.
		return map[string]any{"name": name, "annotations": map[string]any{"readOnlyHint": true}, "inputSchema": map[string]any{"type": "object", "properties": p, "required": required}}
	}
	return []map[string]any{
		tool("list_issues", []string{}, "query", "teamId", "limit"),
		tool("get_issue", []string{"id"}, "id"),
		tool("create_issue", []string{"title", "teamId"}, "title", "teamId", "description"),
		tool("delete_issue", []string{"id"}, "id"),
	}
}

// newStubLinear starts the stub. drop names a tool to leave out of tools/list (a drifted upstream).
func newStubLinear(t *testing.T, drop string) *stubLinear {
	s := &stubLinear{}
	for _, tl := range pinnedTools() {
		if tl["name"] != drop {
			s.tools = append(s.tools, tl)
		}
	}
	s.srv = httptest.NewServer(http.HandlerFunc(s.serve))
	t.Cleanup(s.srv.Close)
	return s
}

func (s *stubLinear) serve(w http.ResponseWriter, r *http.Request) {
	var msg struct {
		ID     any            `json:"id"`
		Method string         `json:"method"`
		Params map[string]any `json:"params"`
	}
	if r.Header.Get("Authorization") != "Bearer "+stubToken {
		http.Error(w, `{"error":"unauthorized"}`, http.StatusUnauthorized)
		return
	}
	if json.NewDecoder(r.Body).Decode(&msg) != nil {
		http.Error(w, "bad json", http.StatusBadRequest)
		return
	}
	if msg.ID == nil {
		w.WriteHeader(http.StatusAccepted)
		return
	}
	if msg.Method == "tools/call" && s.echoRaw(w, r.Header.Get("Authorization"), msg.Params) {
		return
	}
	reply := map[string]any{"jsonrpc": "2.0", "id": msg.ID}
	switch msg.Method {
	case "initialize":
		reply["result"] = map[string]any{"protocolVersion": "2025-03-26", "capabilities": map[string]any{"tools": map[string]any{}}, "serverInfo": map[string]any{"name": "stub-linear"}}
	case "tools/list":
		reply["result"] = map[string]any{"tools": s.tools}
	case "tools/call":
		reply = s.call(reply, r.Header.Get("Authorization"), msg.Params)
	default:
		reply["error"] = map[string]any{"code": -32601, "message": "no such method"}
	}
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(reply)
}

// call records the call and answers it. get_issue with id "echo" fails with an error that echoes the bearer, as a careless
// upstream might: the pod must remove it before the agent sees anything.
func (s *stubLinear) call(reply map[string]any, auth string, p map[string]any) map[string]any {
	name, _ := p["name"].(string)
	args, _ := p["arguments"].(map[string]any)
	s.mu.Lock()
	s.calls = append(s.calls, stubCall{Name: name, Bearer: auth, Args: args})
	s.mu.Unlock()
	switch {
	case name == "get_issue" && args["id"] == "echo":
		reply["error"] = map[string]any{"code": -32000, "message": "credential rejected: " + auth}
	case name == "list_issues":
		reply["result"] = map[string]any{"content": []any{map[string]any{"type": "text", "text": "1 issue"}}, "structuredContent": map[string]any{"issues": []any{map[string]any{"id": "ISS-1", "title": "Login fails"}}}}
	case name == "create_issue":
		reply["result"] = map[string]any{"content": []any{map[string]any{"type": "text", "text": "created"}}, "structuredContent": map[string]any{"issue": map[string]any{"id": "ISS-2", "title": args["title"]}}}
	default:
		reply["result"] = map[string]any{"content": []any{map[string]any{"type": "text", "text": "ok"}}}
	}
	return reply
}

// echoRaw answers get_issue with an id of echo-u, echo-url or echo-b64 the way a careless upstream might fail: an HTTP
// error whose body echoes the bearer \u-escaped in JSON, URL-encoded, or base64-encoded. It records the call.
func (s *stubLinear) echoRaw(w http.ResponseWriter, auth string, p map[string]any) bool {
	args, _ := p["arguments"].(map[string]any)
	id, _ := args["id"].(string)
	if p["name"] != "get_issue" || !strings.HasPrefix(id, "echo-") {
		return false
	}
	s.mu.Lock()
	s.calls = append(s.calls, stubCall{Name: "get_issue", Bearer: auth, Args: args})
	s.mu.Unlock()
	switch id {
	case "echo-u":
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		fmt.Fprintf(w, `{"detail":"credential rejected: %s"}`, uEscaped(auth))
	case "echo-url":
		w.Header().Set("Content-Type", "text/plain")
		w.WriteHeader(http.StatusBadRequest)
		fmt.Fprintf(w, "credential rejected: %s", url.QueryEscape(auth))
	default:
		w.Header().Set("Content-Type", "text/plain")
		w.WriteHeader(http.StatusForbidden)
		fmt.Fprintf(w, "credential rejected: %s", base64.StdEncoding.EncodeToString([]byte(auth)))
	}
	return true
}

// uEscaped writes every character of s as a JSON \u escape.
func uEscaped(s string) string {
	var b strings.Builder
	for _, r := range s {
		fmt.Fprintf(&b, `\u%04x`, r)
	}
	return b.String()
}

// leakIn names the form of the token found in b (raw, \u-escaped, URL-encoded or base64, alone or after "Bearer "), or "".
func leakIn(b []byte) string {
	s := string(b)
	forms := map[string]string{"raw": stubToken, "u-escaped": uEscaped(stubToken), "query-escaped": url.QueryEscape(stubToken), "path-escaped": url.PathEscape(stubToken)}
	for _, v := range []string{stubToken, "Bearer " + stubToken} {
		forms["base64 of "+v[:6]] = base64.StdEncoding.EncodeToString([]byte(v))
		forms["base64url of "+v[:6]] = base64.URLEncoding.EncodeToString([]byte(v))
	}
	for name, f := range forms {
		if strings.Contains(s, f) || strings.Contains(s, strings.ReplaceAll(f, `\`, `\\`)) {
			return name
		}
	}
	return ""
}

// seen returns the recorded tools/call requests for one tool name.
func (s *stubLinear) seen(name string) []stubCall {
	s.mu.Lock()
	defer s.mu.Unlock()
	return slices.DeleteFunc(slices.Clone(s.calls), func(c stubCall) bool { return c.Name != name })
}
