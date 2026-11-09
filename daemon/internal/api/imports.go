package api

import (
	"fmt"
	"net/http"
	"net/url"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// The calls `jellymoon import commonly` makes as the person: the existing create-pod, post and feed routes, under their jmu_ token.

// CreatePod creates a pod owned by the user the token belongs to.
func (c *Client) CreatePod(userToken, name string) (protocol.Pod, error) {
	var out protocol.Pod
	return out, c.post("/v1/pods", userToken, protocol.CreatePodRequest{Name: name}, &out)
}

// PostMessage posts to a pod as the user the token belongs to.
func (c *Client) PostMessage(userToken, podID string, in protocol.PostInput) (protocol.PostOutput, error) {
	var out protocol.PostOutput
	return out, c.post("/v1/pods/"+url.PathEscape(podID)+"/messages", userToken, in, &out)
}

// Feed reads a pod's messages with their bodies after a sequence number (at most 50 per call).
func (c *Client) Feed(userToken, podID string, sinceSeq int64) (protocol.ReadOutput, error) {
	var out protocol.ReadOutput
	q := fmt.Sprintf("?withBody=true&limit=50&sinceSeq=%d", sinceSeq)
	return out, c.call(http.MethodGet, "/v1/pods/"+url.PathEscape(podID)+"/feed"+q, userToken, nil, &out)
}
