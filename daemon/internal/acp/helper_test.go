package acp

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"testing"
	"time"
)

// The tests start this test binary as the agent: with ACP_AGENT set, TestMain plays an ACP agent on stdio instead of running tests.
// ACP_AGENT is a mode (see agentMain) and ACP_REC a file the agent appends one event name per line to.
func TestMain(m *testing.M) {
	if mode := os.Getenv("ACP_AGENT"); mode != "" {
		agentMain(mode, os.Getenv("ACP_REC"))
		os.Exit(0)
	}
	os.Exit(m.Run())
}

func note(rec, event string) {
	f, err := os.OpenFile(rec, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600)
	if err == nil {
		fmt.Fprintln(f, event)
		f.Close()
	}
}

func send(v map[string]any) {
	v["jsonrpc"] = "2.0"
	b, _ := json.Marshal(v)
	os.Stdout.Write(append(b, '\n'))
}

// agentMain modes:
//
//	ask:<kind,kind>  on the prompt, ask permission offering one option per kind (its id is the kind), record the answer, end the turn
//	ask-other        on the prompt, make a request the client does not serve, record the error code, end the turn
//	cancel           on the prompt, wait for session/cancel, then end the turn cancelled and exit
//	cancel:<reason>  like cancel, but the turn ends with that stopReason instead of "cancelled" (an agent that answers a stop with a refusal)
//	deaf             on the prompt, ignore everything; a SIGTERM is recorded and ends it
//	stubborn         like deaf, but it ignores SIGTERM, so only SIGKILL ends it before its 20 s are up
//	stop:<reason>    on the prompt, end the turn with that stopReason ("none" sends a result with no stopReason)
//	exit-on-prompt  on the prompt, exit without answering and with no stop asked
//	exit-on-cancel   on session/cancel, exit without answering the prompt
//	error-on-cancel  on session/cancel, answer the prompt with a JSON-RPC error
func agentMain(mode, rec string) {
	term := make(chan os.Signal, 1)
	signal.Notify(term, syscall.SIGTERM)
	if mode == "stubborn" {
		signal.Ignore(syscall.SIGTERM)
	}
	go func() { <-term; note(rec, "term"); os.Exit(143) }()
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		var m struct {
			ID     json.RawMessage `json:"id"`
			Method string          `json:"method"`
			Result json.RawMessage `json:"result"`
			Error  *struct{ Code int }
		}
		if json.Unmarshal(sc.Bytes(), &m) != nil {
			continue
		}
		switch m.Method {
		case "initialize":
			send(map[string]any{"id": m.ID, "result": map[string]any{"protocolVersion": 1}})
		case "session/new":
			send(map[string]any{"id": m.ID, "result": map[string]any{"sessionId": "sess_t"}})
		case "session/cancel":
			note(rec, "cancel")
			if strings.HasPrefix(mode, "cancel") {
				cancelWith(mode)
				return
			}
			switch mode {
			case "error-on-cancel":
				send(map[string]any{"id": 3, "error": map[string]any{"code": -32000, "message": "boom"}})
				fallthrough
			case "exit-on-cancel":
				return
			}
		case "session/prompt":
			note(rec, "prompt")
			switch {
			case strings.HasPrefix(mode, "ask:"):
				ask(sc, strings.Split(strings.TrimPrefix(mode, "ask:"), ","), rec)
				send(map[string]any{"id": m.ID, "result": map[string]any{"stopReason": "end_turn"}})
				return
			case strings.HasPrefix(mode, "stop:"):
				stopWith(m.ID, strings.TrimPrefix(mode, "stop:"))
				return
			case mode == "exit-on-prompt":
				return
			case mode == "ask-other":
				askOther(sc, m.ID, rec)
				return
			}
		}
	}
	if mode == "deaf" || mode == "stubborn" {
		time.Sleep(20 * time.Second) // a stuck agent does not leave when its stdin closes; this one gives up after 20 s
	}
}

func ask(sc *bufio.Scanner, kinds []string, rec string) {
	var opts []map[string]any
	for _, k := range kinds {
		opts = append(opts, map[string]any{"optionId": k, "name": k, "kind": k})
	}
	send(map[string]any{"id": 99, "method": "session/request_permission", "params": map[string]any{
		"sessionId": "sess_t", "toolCall": map[string]any{"toolCallId": "call_1", "kind": "execute", "title": "rm -rf SECRET_ARG"}, "options": opts}})
	sc.Scan()
	var r struct {
		Result struct {
			Outcome struct{ Outcome, OptionId string }
		}
	}
	_ = json.Unmarshal(sc.Bytes(), &r)
	a := r.Result.Outcome.Outcome
	if a == "selected" {
		a = r.Result.Outcome.OptionId
	}
	note(rec, "answer:"+a)
}

// cancelWith winds the turn down (that takes a moment, so a client that signals at once is caught), then answers the outstanding
// prompt (id 3): mode "cancel" with stopReason cancelled, mode "cancel:<reason>" with that reason.
func cancelWith(mode string) {
	time.Sleep(300 * time.Millisecond)
	reason := strings.TrimPrefix(mode, "cancel:")
	if mode == "cancel" {
		reason = "cancelled"
	}
	stopWith(json.RawMessage("3"), reason)
}

// stopWith answers the prompt with a result carrying stopReason reason, or none at all when reason is "none".
func stopWith(id json.RawMessage, reason string) {
	res := map[string]any{}
	if reason != "none" {
		res["stopReason"] = reason
	}
	send(map[string]any{"id": id, "result": res})
}

// askOther makes a request the client does not serve, records the error code it gets, and ends the turn.
func askOther(sc *bufio.Scanner, id json.RawMessage, rec string) {
	send(map[string]any{"id": 77, "method": "fs/read_text_file", "params": map[string]any{}})
	sc.Scan()
	var r struct{ Error *struct{ Code int } }
	_ = json.Unmarshal(sc.Bytes(), &r)
	note(rec, fmt.Sprintf("other-error:%d", r.Error.Code))
	send(map[string]any{"id": id, "result": map[string]any{"stopReason": "end_turn"}})
}
