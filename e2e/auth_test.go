package e2e

import "testing"

// TestRunTokenOtherPod: a run token belongs to one pod. On another pod's routes it gets 403 forbidden, whatever the route,
// before the pod is even looked at; on its own pod's routes it works.
func TestRunTokenOtherPod(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "own pod")
	var other struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "elsewhere"}, &other), "create a second pod")
	for _, c := range []struct{ method, path string }{
		{"GET", "/v1/pods/" + other.ID + "/me"},
		{"GET", "/v1/pods/" + other.ID + "/tasks"},
		{"GET", "/v1/pods/" + other.ID + "/feed"},
		{"POST", "/v1/pods/" + other.ID + "/messages"},
	} {
		var in any
		if c.method == "POST" {
			in = post(nil)
		}
		w.wantErr(w.runCall(r, c.method, c.path, in, nil), 403, "forbidden", "run token on another pod: "+c.method+" "+c.path)
	}
	w.must(w.runCall(r, "GET", w.podPath("me"), nil, nil), "the same run token on its own pod")
}
