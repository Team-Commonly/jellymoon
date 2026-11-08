package acp

import "encoding/json"

type option struct {
	ID   string `json:"optionId"`
	Kind string `json:"kind"`
}

// pick returns the first option of the first kind in order that the request offers. An option with no optionId cannot be answered.
func pick(opts []option, order ...string) (option, bool) {
	for _, k := range order {
		for _, o := range opts {
			if o.Kind == k && o.ID != "" {
				return o, true
			}
		}
	}
	return option{}, false
}

// serve answers one request the agent made to the client. session/request_permission follows the policy in ADR 0007; any other
// method gets -32601, as it always has.
func (c *conn) serve(m message) {
	if m.Method != "session/request_permission" {
		_ = c.write(map[string]any{"id": m.ID, "error": map[string]any{"code": -32601, "message": "not supported by this client"}})
		return
	}
	var p struct {
		ToolCall struct {
			ID   string `json:"toolCallId"`
			Kind string `json:"kind"`
		} `json:"toolCall"`
		Options []option `json:"options"`
	}
	_ = json.Unmarshal(m.Params, &p) // a request we cannot read offers no options, so it is answered cancelled
	order := []string{"reject_once", "reject_always"}
	if c.allow {
		order = []string{"allow_once", "reject_once", "reject_always"} // never allow_always: the daemon could not revoke it
	}
	if c.cancelling {
		order = nil // the turn is being abandoned, and ACP has every pending request answered cancelled
	}
	outcome, answer := map[string]any{"outcome": "cancelled"}, "cancelled"
	if o, ok := pick(p.Options, order...); ok {
		outcome, answer = map[string]any{"outcome": "selected", "optionId": o.ID}, o.Kind
	}
	// The log names the tool call by id and kind and the answer by kind. The call's title and input can hold a command line or a secret.
	if c.logf != nil {
		c.logf("permission request: tool call %q (kind %q) answered %s", p.ToolCall.ID, p.ToolCall.Kind, answer)
	}
	_ = c.write(map[string]any{"id": m.ID, "result": map[string]any{"outcome": outcome}})
}
