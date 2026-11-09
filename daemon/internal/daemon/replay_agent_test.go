package daemon

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"testing"
	"time"
)

// The recording (testdata/acp/claude-slice.jsonl) is one real Claude Code run through the claude-agent-acp adapter, written by
// scripts/record-acp.mjs. Its first line is a header; every other line is an ACP line the daemon sent ("in"), an ACP line the agent
// sent ("out"), or one HTTP exchange the agent had with the pod's MCP server ("mcp").
type entry struct {
	T      string          `json:"t"`
	Dir    string          `json:"dir"`
	Msg    json.RawMessage `json:"msg"`
	Method string          `json:"method"`
	Req    json.RawMessage `json:"req"`
	Status int             `json:"status"`
	Res    json.RawMessage `json:"res"`
}

type rpc struct {
	ID     json.RawMessage `json:"id"`
	Method string          `json:"method"`
	Params json.RawMessage `json:"params"`
	Result json.RawMessage `json:"result"`
	Error  json.RawMessage `json:"error"`
}

func loadRecording(path string) ([]entry, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var out []entry
	for i, line := range strings.Split(strings.TrimSpace(string(raw)), "\n") {
		var e entry
		if err := json.Unmarshal([]byte(line), &e); err != nil {
			return nil, fmt.Errorf("%s line %d: %w", path, i+1, err)
		}
		if i > 0 {
			out = append(out, e)
		}
	}
	return out, nil
}

// replayAgent and replayReport are how TestReplayClaude starts this test binary as the agent: the daemon runs the program
// JELLYMOON_HARNESS_CLAUDE_CODE names, and that program is this binary running only TestReplayAgent. The report file is where a
// failed agent says why, since the daemon gives its agent no stderr.
var (
	replayAgent  = flag.String("replay.agent", "", "(internal) play the recording at this path as the ACP agent")
	replayReport = flag.String("replay.report", "", "(internal) where the replaying agent writes why it failed")
)

func TestReplayAgent(t *testing.T) {
	if *replayAgent == "" {
		t.Skip("runs only as the agent TestReplayClaude starts")
	}
	ents, err := loadRecording(*replayAgent)
	if err == nil {
		err = (&player{sc: newScanner(os.Stdin), out: os.Stdout, ids: map[string]json.RawMessage{}}).play(ents)
	}
	if err != nil {
		_ = os.WriteFile(*replayReport, []byte(err.Error()), 0o600)
		os.Exit(1)
	}
	os.Exit(0) // a test binary that returns prints PASS on the ACP stream
}

func newScanner(r io.Reader) *bufio.Scanner {
	sc := bufio.NewScanner(r)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	return sc
}

// player plays the agent's side of the recording in order. What the daemon sends is read and checked against the recording; what the
// agent sent is written back, with each response carrying the id of the request it answers; each MCP exchange is re-issued against
// the server and headers session/new named, so the pod sees the calls the real agent made, with the run token this run was given.
type player struct {
	sc      *bufio.Scanner
	out     io.Writer
	ids     map[string]json.RawMessage // a request id in the recording -> the id the daemon used
	url     string
	headers []struct{ Name, Value string }
}

func (p *player) play(ents []entry) error {
	for i, e := range ents {
		var err error
		switch {
		case e.T == "acp" && e.Dir == "in":
			err = p.expect(e.Msg)
		case e.T == "acp":
			err = p.send(e.Msg)
		case e.T == "mcp":
			err = p.call(e)
		}
		if err != nil {
			return fmt.Errorf("recording entry %d: %w", i+2, err)
		}
	}
	return nil
}

// expect reads the next line the daemon sent and checks it is the one the recording has.
func (p *player) expect(want json.RawMessage) error {
	if !p.sc.Scan() {
		return errors.New("the daemon closed the agent's stdin before sending what the recording has next")
	}
	var got, rec rpc
	if err := json.Unmarshal(p.sc.Bytes(), &got); err != nil {
		return err
	}
	_ = json.Unmarshal(want, &rec)
	if got.Method != rec.Method || (got.Result == nil) != (rec.Result == nil) {
		return fmt.Errorf("the daemon sent %q (%s), the recording has %q", got.Method, p.sc.Text(), rec.Method)
	}
	if rec.Method != "" {
		p.ids[string(rec.ID)] = got.ID
	}
	switch rec.Method {
	case "session/new":
		return p.learn(got.Params)
	case "session/prompt":
		return samePrompt(got.Params, rec.Params)
	}
	return nil
}

// learn keeps the MCP server the daemon named in session/new.
func (p *player) learn(params json.RawMessage) error {
	var n struct {
		McpServers []struct {
			URL     string
			Headers []struct{ Name, Value string }
		}
	}
	if json.Unmarshal(params, &n) != nil || len(n.McpServers) != 1 {
		return fmt.Errorf("session/new did not carry exactly one MCP server: %s", params)
	}
	p.url, p.headers = n.McpServers[0].URL, n.McpServers[0].Headers
	return nil
}

// samePrompt: the daemon passes the pod's brief through unchanged. (The sessionId is the agent's own, so it is not compared.)
func samePrompt(got, rec json.RawMessage) error {
	var g, r struct{ Prompt json.RawMessage }
	_ = json.Unmarshal(got, &g)
	_ = json.Unmarshal(rec, &r)
	if !bytes.Equal(g.Prompt, r.Prompt) {
		return fmt.Errorf("the daemon's prompt %s is not the recorded %s", g.Prompt, r.Prompt)
	}
	return nil
}

// send writes one line the agent sent. A response takes the id of the daemon's request.
func (p *player) send(msg json.RawMessage) error {
	var m map[string]json.RawMessage
	if err := json.Unmarshal(msg, &m); err != nil {
		return err
	}
	if _, ok := m["method"]; !ok {
		id, ok := p.ids[string(m["id"])]
		if !ok {
			return fmt.Errorf("the recording answers request %s, which the daemon never sent", m["id"])
		}
		m["id"] = id
	}
	b, _ := json.Marshal(m)
	_, err := p.out.Write(append(b, '\n'))
	return err
}

// call re-issues one recorded MCP exchange and reads the answer; the pod decides what to say.
func (p *player) call(e entry) error {
	var body io.Reader
	if e.Method == http.MethodPost {
		body = bytes.NewReader(e.Req)
	}
	req, err := http.NewRequest(e.Method, p.url, body)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json, text/event-stream")
	for _, h := range p.headers {
		req.Header.Set(h.Name, h.Value)
	}
	resp, err := (&http.Client{Timeout: 10 * time.Second}).Do(req)
	if err != nil {
		return err
	}
	_, _ = io.Copy(io.Discard, resp.Body)
	return resp.Body.Close()
}
