package e2e

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"testing"
	"time"
)

// Local wire structs. daemon/internal/protocol is Go-internal and cannot be imported from e2e/.
type Actor struct{ Kind, ID, DisplayName string }

type Claim struct {
	TaskID, RunID, LeaseUntil string // TaskID: task.claimed only
	By                        Actor
}

type Task struct {
	ID, Title, Status, Assignee string
	Claim                       *Claim
}

type Released struct {
	TaskID, RunID, Cause, Reason string
	By                           *Actor
}

type Wake struct {
	WakeID, AgentID, Reason string
	Refs                    struct{ TaskID string }
}

type ErrorBody struct{ Code, Message, Ref string }

type res struct {
	Status int
	Err    ErrorBody
}

func (r res) ok() bool { return r.Status >= 200 && r.Status < 300 }

type client struct {
	t    *testing.T
	base string
	hc   *http.Client
}

// do sends a JSON request. It is safe to call from goroutines: it reports through t.Errorf and
// returns Status -1 on a transport failure.
func (c *client) do(method, path, tok string, in, out any) res {
	var body io.Reader
	if in != nil {
		b, _ := json.Marshal(in)
		body = bytes.NewReader(b)
	}
	req, _ := http.NewRequest(method, c.base+path, body)
	req.Header.Set("Content-Type", "application/json")
	if tok != "" {
		req.Header.Set("Authorization", "Bearer "+tok)
	}
	resp, err := c.hc.Do(req)
	if err != nil {
		c.t.Errorf("%s %s: %v", method, path, err)
		return res{Status: -1}
	}
	defer resp.Body.Close()
	raw, _ := io.ReadAll(resp.Body)
	r := res{Status: resp.StatusCode}
	switch {
	case r.ok() && out != nil:
		if err := json.Unmarshal(raw, out); err != nil {
			c.t.Errorf("%s %s: response is not the documented JSON: %v (%.200s)", method, path, err, raw)
		}
	case !r.ok():
		_ = json.Unmarshal(raw, &r.Err)
	}
	return r
}

func newClient(t *testing.T, base string) *client {
	return &client{t: t, base: base, hc: &http.Client{Timeout: 10 * time.Second}}
}

func ms(ts string) int64 {
	p, err := time.Parse(time.RFC3339Nano, ts)
	if err != nil {
		return -1
	}
	return p.UnixMilli()
}
