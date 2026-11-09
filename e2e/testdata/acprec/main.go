// Command acprec is the recording tee for a real ACP agent (docs/development/record-acp.md). The daemon starts it as the harness
// program: `acprec <log> <agent program> [args]`. It runs the real agent, copies every ACP line both ways, and points the MCP server
// session/new names at a local proxy that forwards to the real URL, so the agent's calls to the pod are logged in order with the
// ACP lines. The log keeps bodies only (no header, so no token), but it is raw: scripts/record-acp.mjs scrubs it, and the raw file
// is never kept. Not a model, and not part of the product.
package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"os/exec"
	"sync"
)

var (
	mu  sync.Mutex
	out *os.File
)

// note appends one log line: {"t":"acp","dir":"in"|"out","msg":...} or {"t":"mcp","method":...,"req":...,"status":...,"res":...}.
func note(v map[string]any) {
	b, _ := json.Marshal(v)
	mu.Lock()
	defer mu.Unlock()
	_, _ = out.Write(append(b, '\n'))
}

func raw(b []byte) json.RawMessage {
	if json.Valid(b) {
		return b
	}
	q, _ := json.Marshal(string(b))
	return q
}

// proxy forwards to target and logs each exchange when it completes.
func proxy(target *url.URL) string {
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		os.Exit(3)
	}
	rp := &httputil.ReverseProxy{Rewrite: func(pr *httputil.ProxyRequest) {
		pr.SetURL(target)
		pr.Out.URL.Path, pr.Out.URL.RawPath = target.Path, target.RawPath
		in, _ := io.ReadAll(pr.In.Body)
		pr.Out.Body = io.NopCloser(bytes.NewReader(in))
		pr.Out.GetBody = func() (io.ReadCloser, error) { return io.NopCloser(bytes.NewReader(in)), nil }
		pr.Out.Header.Del("Accept-Encoding")
	}}
	rp.ModifyResponse = func(resp *http.Response) error {
		body, _ := io.ReadAll(resp.Body)
		resp.Body = io.NopCloser(bytes.NewReader(body))
		var in []byte
		if g, err := resp.Request.GetBody(); err == nil {
			in, _ = io.ReadAll(g)
		}
		note(map[string]any{"t": "mcp", "method": resp.Request.Method, "req": raw(in), "status": resp.StatusCode, "res": raw(body)})
		return nil
	}
	go func() { _ = http.Serve(l, rp) }()
	return "http://" + l.Addr().String() + "/mcp"
}

// rewrite points every MCP server in a session/new line at the proxy.
func rewrite(line []byte) []byte {
	var m map[string]json.RawMessage
	if json.Unmarshal(line, &m) != nil || string(m["method"]) != `"session/new"` {
		return line
	}
	var p map[string]json.RawMessage
	var servers []map[string]any
	if json.Unmarshal(m["params"], &p) != nil || json.Unmarshal(p["mcpServers"], &servers) != nil {
		return line
	}
	for _, s := range servers {
		if u, err := url.Parse(s["url"].(string)); err == nil {
			s["url"] = proxy(u)
		}
	}
	p["mcpServers"], _ = json.Marshal(servers)
	m["params"], _ = json.Marshal(p)
	b, _ := json.Marshal(m)
	return b
}

func main() {
	if len(os.Args) < 3 {
		os.Exit(2)
	}
	var err error
	if out, err = os.OpenFile(os.Args[1], os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o600); err != nil {
		os.Exit(2)
	}
	cmd := exec.Command(os.Args[2], os.Args[3:]...)
	cmd.Stderr = os.Stderr
	in, _ := cmd.StdinPipe()
	agentOut, _ := cmd.StdoutPipe()
	if cmd.Start() != nil {
		os.Exit(2)
	}
	done := make(chan struct{})
	go func() { // agent to daemon
		lines(agentOut, func(b []byte) {
			note(map[string]any{"t": "acp", "dir": "out", "msg": raw(b)})
			_, _ = os.Stdout.Write(append(b, '\n'))
		})
		close(done)
	}()
	lines(os.Stdin, func(b []byte) { // daemon to agent
		note(map[string]any{"t": "acp", "dir": "in", "msg": raw(b)})
		_, _ = in.Write(append(rewrite(b), '\n'))
	})
	_ = in.Close()
	<-done
	_ = cmd.Wait()
}

// lines calls f with each newline-ended line r yields, however long.
func lines(r io.Reader, f func([]byte)) {
	var buf []byte
	one := make([]byte, 1<<16)
	for {
		n, err := r.Read(one)
		buf = append(buf, one[:n]...)
		for i := bytes.IndexByte(buf, '\n'); i >= 0; i = bytes.IndexByte(buf, '\n') {
			if i > 0 {
				f(append([]byte(nil), buf[:i]...))
			}
			buf = buf[i+1:]
		}
		if err != nil {
			return
		}
	}
}
