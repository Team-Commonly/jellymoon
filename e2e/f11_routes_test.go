package e2e

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
)

// toolCallRoutes: the pod's count of MCP calls is fed only by the MCP adapter. No path on the public host reaches it, direct or by traversal.
func (f *f11) toolCallRoutes() error {
	pod, run := "/v1/pods/"+f.pod, f.run.RunID
	paths := []string{
		"/_in/tool-call", "/_in/../_in/tool-call", "/v1/_in/tool-call", pod + "/_in/tool-call", pod + "/%2e%2e/_in/tool-call", pod + "/..%2f_in%2ftool-call",
		pod + "/tool-call", pod + "/tool_call", pod + "/tool-calls", pod + "/runs/" + run + "/tool-call", pod + "/runs/" + run + "/tool-calls",
		"/v1/runs/" + run + "/tool-call", "/v1/tool-call", "/tool-call", "/mcp/tool-call",
	}
	for _, p := range paths {
		for _, method := range []string{"POST", "GET"} {
			for who, tok := range map[string]string{"run": f.run.RunToken, "owner": f.owner.tok} {
				req, _ := http.NewRequest(method, f.srv.base()+p, bytes.NewReader([]byte(`{"runId":"`+run+`"}`)))
				req.Header.Set("Authorization", "Bearer "+tok)
				resp, err := http.DefaultClient.Do(req)
				if err != nil {
					return err
				}
				out, _ := io.ReadAll(resp.Body)
				resp.Body.Close()
				if resp.StatusCode != 404 {
					return fmt.Errorf("%s %s with a %s token: HTTP %d %s, want 404", method, p, who, resp.StatusCode, out)
				}
			}
		}
	}
	// And nothing was counted: the next post's receipt has the calls the agent made (the ask included) and no more.
	summary := "After the probes"
	if st, raw := f.httpAs(f.run.RunToken, map[string]any{"summary": summary, "kind": "fyi", "status": "done"}, nil); st != 200 {
		return fmt.Errorf("post after the probes: HTTP %d %s", st, raw)
	}
	m, err := f.storedWith(summary)
	if err != nil || m.Receipt == nil || m.Receipt.ToolCalls != askAt+1 {
		return fmt.Errorf("after the probes the receipt is %+v (err %v), want toolCalls %d", m.Receipt, err, askAt+1)
	}
	return nil
}
