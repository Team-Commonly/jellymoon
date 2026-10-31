// Command fakeagent is the fake ACP agent the e2e suite hands to the daemon: JSON-RPC 2.0 over stdio, one message per line.
// It is NOT a model. It writes what it received (argv, env, the ACP requests) to the file named by its first argument,
// calls jm_whoami through the MCP server session/new gave it, records the answer, and exits. After the record path, the
// arguments are: "hold" (record, then stay up, ignoring a closed stdin, to stand for an agent that will not leave by itself)
// and "script=<file>" (a JSON list of {"tool", "arguments"}: the agent makes those MCP calls in order instead of jm_whoami,
// and records each reply under Calls. It never decides anything from a reply: a script is the whole of its behaviour).
package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"os"
	"strings"
	"time"
)

type header struct{ Name, Value string }

type server struct {
	Type    string
	Name    string
	URL     string
	Headers []header
}

// record is the file the test reads. Fields are written as they are learned, so a crash still leaves the earlier ones.
type record struct {
	PID         int
	Argv        []string
	Env         []string
	Initialize  json.RawMessage
	SessionNew  json.RawMessage
	Prompt      json.RawMessage
	Whoami      json.RawMessage
	WhoamiError string
	Calls       []callRec
}

// callRec is one scripted MCP call and the server's whole answer to it.
type callRec struct {
	Tool  string
	Reply json.RawMessage
	Error string
}

type step struct {
	Tool      string
	Arguments map[string]any
}

type request struct {
	ID     json.RawMessage `json:"id"`
	Method string          `json:"method"`
	Params json.RawMessage `json:"params"`
}

var (
	rec  record
	path string
)

func save() {
	raw, _ := json.Marshal(rec)
	tmp := path + ".tmp"
	if os.WriteFile(tmp, raw, 0o600) == nil {
		_ = os.Rename(tmp, path)
	}
}

func reply(id json.RawMessage, result any) {
	b, _ := json.Marshal(map[string]any{"jsonrpc": "2.0", "id": id, "result": result})
	os.Stdout.Write(append(b, '\n'))
}

// call posts one JSON-RPC message to the MCP server with the headers session/new gave, and returns the body.
func call(s server, body map[string]any) (json.RawMessage, error) {
	raw, _ := json.Marshal(body)
	req, err := http.NewRequest(http.MethodPost, s.URL, bytes.NewReader(raw))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json, text/event-stream")
	for _, h := range s.Headers {
		req.Header.Set(h.Name, h.Value)
	}
	resp, err := (&http.Client{Timeout: 10 * time.Second}).Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	out, _ := io.ReadAll(resp.Body)
	return out, nil
}

func whoami(params json.RawMessage) {
	var p struct{ McpServers []server }
	if err := json.Unmarshal(params, &p); err != nil || len(p.McpServers) == 0 {
		rec.WhoamiError = "session/new carried no MCP server"
		return
	}
	s := p.McpServers[0]
	if _, err := call(s, map[string]any{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": map[string]any{
		"protocolVersion": "2025-03-26", "capabilities": map[string]any{}, "clientInfo": map[string]any{"name": "fakeagent", "version": "0"}}}); err != nil {
		rec.WhoamiError = err.Error()
		return
	}
	out, err := call(s, map[string]any{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": map[string]any{"name": "jm_whoami", "arguments": map[string]any{}}})
	if err != nil {
		rec.WhoamiError = err.Error()
		return
	}
	rec.Whoami = out
}

// script makes the scripted calls, one per tools/call, after one initialize.
func script(params json.RawMessage, file string) {
	var p struct{ McpServers []server }
	var steps []step
	raw, err := os.ReadFile(file)
	if err == nil {
		err = json.Unmarshal(raw, &steps)
	}
	if err != nil || json.Unmarshal(params, &p) != nil || len(p.McpServers) == 0 {
		rec.WhoamiError = "the script or the MCP server is unusable"
		return
	}
	s := p.McpServers[0]
	if _, err := call(s, map[string]any{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": map[string]any{
		"protocolVersion": "2025-03-26", "capabilities": map[string]any{}, "clientInfo": map[string]any{"name": "fakeagent", "version": "0"}}}); err != nil {
		rec.WhoamiError = err.Error()
		return
	}
	for i, st := range steps {
		out, err := call(s, map[string]any{"jsonrpc": "2.0", "id": i + 2, "method": "tools/call", "params": map[string]any{"name": st.Tool, "arguments": st.Arguments}})
		c := callRec{Tool: st.Tool, Reply: out}
		if err != nil {
			c.Error = err.Error()
		}
		rec.Calls = append(rec.Calls, c)
		save()
	}
}

func main() {
	if len(os.Args) < 2 {
		os.Exit(2)
	}
	path = os.Args[1]
	hold, scripted := false, ""
	for _, a := range os.Args[2:] {
		if a == "hold" {
			hold = true
		}
		if f, ok := strings.CutPrefix(a, "script="); ok {
			scripted = f
		}
	}
	rec.PID, rec.Argv, rec.Env = os.Getpid(), os.Args, os.Environ()
	save()
	sc := bufio.NewScanner(os.Stdin)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		var r request
		if json.Unmarshal(sc.Bytes(), &r) != nil {
			continue
		}
		switch r.Method {
		case "initialize":
			rec.Initialize = r.Params
			reply(r.ID, map[string]any{"protocolVersion": 1, "agentCapabilities": map[string]any{}})
		case "session/new":
			rec.SessionNew = r.Params
			reply(r.ID, map[string]any{"sessionId": "sess_fake"})
		case "session/prompt":
			rec.Prompt = r.Params
			if scripted != "" {
				script(rec.SessionNew, scripted)
			} else {
				whoami(rec.SessionNew)
			}
			save()
			if hold {
				select {}
			}
			reply(r.ID, map[string]any{"stopReason": "end_turn"})
			return
		}
		save()
	}
	if hold {
		select {}
	}
}
