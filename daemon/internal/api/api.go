// Package api is the daemon's HTTP client for the Worker (docs/protocol.md, Worker HTTP API). A Client can only be made for a
// server the daemon trusts with a token: https, or http on localhost or 127.0.0.1. Nothing is sent before that check, and
// no error or log line this package builds carries a token.
package api

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"runtime"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// CheckServer parses a server URL and refuses everything but https, or http on localhost or 127.0.0.1. protocol.md puts this
// check on the daemon: the schema's pattern is not enforced by Go. It returns scheme://host[:port] with no path.
func CheckServer(raw string) (*url.URL, error) {
	u, err := url.Parse(raw)
	if err != nil || u.Host == "" || u.User != nil {
		return nil, fmt.Errorf("the server URL is not a plain https URL with a host")
	}
	host := u.Hostname()
	local := u.Scheme == "http" && (host == "localhost" || host == "127.0.0.1")
	if u.Scheme != "https" && !local {
		return nil, fmt.Errorf("refusing server %s://%s: it must be https, or http on localhost or 127.0.0.1", u.Scheme, u.Host)
	}
	return &url.URL{Scheme: u.Scheme, Host: u.Host}, nil
}

// Client calls the Worker.
type Client struct {
	Base *url.URL
	hc   *http.Client
}

// New checks the server URL and makes a client that does not follow redirects, so a token never travels to another host.
func New(raw string) (*Client, error) {
	base, err := CheckServer(raw)
	if err != nil {
		return nil, err
	}
	hc := &http.Client{Timeout: 15 * time.Second, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	return &Client{Base: base, hc: hc}, nil
}

func (c *Client) post(path, token string, in, out any) error {
	body, err := json.Marshal(in)
	if err != nil {
		return err
	}
	req, err := http.NewRequest(http.MethodPost, c.Base.String()+path, bytes.NewReader(body))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	resp, err := c.hc.Do(req)
	if err != nil {
		return fmt.Errorf("POST %s: %w", path, redact(err))
	}
	defer resp.Body.Close()
	raw, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if resp.StatusCode < 200 || resp.StatusCode > 299 {
		var e protocol.ErrorBody
		if json.Unmarshal(raw, &e) == nil {
			return fmt.Errorf("POST %s: HTTP %d %s: %s", path, resp.StatusCode, e.Code, e.Message)
		}
		return fmt.Errorf("POST %s: HTTP %d", path, resp.StatusCode)
	}
	return json.Unmarshal(raw, out)
}

// redact drops the request URL from a transport error; the error text of a failed request is the URL and the cause.
func redact(err error) error {
	if ue, ok := err.(*url.Error); ok {
		return ue.Err
	}
	return err
}

// RegisterComputer registers this machine under the user the jmu_ token belongs to.
func (c *Client) RegisterComputer(userToken string) (protocol.RegisterComputerResponse, error) {
	var out protocol.RegisterComputerResponse
	name, _ := os.Hostname()
	if name == "" {
		name = "computer"
	}
	req := protocol.RegisterComputerRequest{Name: name[:min(len(name), 80)], OS: protocol.OS(runtime.GOOS), Arch: protocol.Arch(runtime.GOARCH)}
	if !req.OS.Valid() || !req.Arch.Valid() {
		return out, fmt.Errorf("%s/%s is not a platform the protocol knows", runtime.GOOS, runtime.GOARCH)
	}
	return out, c.post("/v1/computers", userToken, req, &out)
}

// RegisterAgent registers an agent of this computer in a pod.
func (c *Client) RegisterAgent(computerToken, podID string, harness protocol.Harness, displayName string) (protocol.AgentID, error) {
	var out protocol.RegisterAgentResponse
	err := c.post("/v1/pods/"+url.PathEscape(podID)+"/agents", computerToken, protocol.RegisterAgentRequest{Harness: harness, DisplayName: displayName}, &out)
	return out.AgentID, err
}

// PodSocket is the pod socket's URL, as an http(s) URL for the ws package to upgrade.
func (c *Client) PodSocket(podID string) *url.URL {
	return c.Base.JoinPath("v1", "pods", podID, "ws")
}
