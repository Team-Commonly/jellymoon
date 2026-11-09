package e2e

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
)

const impSentinel = "cm_e2eSENTINELtoken0123456789"

type obj = map[string]any

// cmy is a fixture Commonly instance: the shapes of the real routes (CMY backend routes), and a record of every request it saw.
type cmy struct {
	me       obj
	pods     []obj
	msgs     map[string][]obj
	grants   []obj
	integ    map[string][]obj
	mu       sync.Mutex
	reqs     []string
	cur, max atomic.Int32
	failMsg  func(pod string, call int) (status int, retryAfter string)
	calls    map[string]int
}

func cmMember(id, name string, bot bool) obj {
	return obj{"_id": id, "username": name, "isBot": bot, "profilePicture": "https://img.example/" + id + ".png"}
}

func cmMsg(id, uid, name, text, at string) obj {
	return obj{"id": id, "_id": id, "user_id": uid, "username": name, "content": text, "messageType": "text", "createdAt": at}
}

func newCmy() *cmy {
	me := cmMember("u_me", "Me Myself", false)
	me["role"], me["email"] = "user", "me@example.test"
	bob, bot := cmMember("u_bob", "Bob Builder", false), cmMember("a_bot", "Helper Bot", true)
	bob["email"] = "bob@example.test"
	c := &cmy{me: me, msgs: map[string][]obj{}, integ: map[string][]obj{}, calls: map[string]int{}}
	c.pods = []obj{
		{"_id": "p_team", "name": "Team room", "type": "team", "members": []obj{me, bob, bot}},
		{"_id": "p_dm", "name": "Me and Helper", "type": "agent-room", "members": []obj{me, bot}},
		{"_id": "p_admin", "name": "Admin room", "type": "agent-admin", "members": []obj{me}},
		{"_id": "p_foreign", "name": "Others DM", "type": "agent-dm", "members": []obj{cmMember("a_x", "X", true), cmMember("a_y", "Y", true)}},
		{"_id": "p_study", "name": "Study", "type": "study", "members": []obj{me, bob}},
	}
	c.msgs["p_team"] = []obj{
		cmMsg("m1", "u_bob", "Bob Builder", "first", "2026-01-01T10:00:00.000Z"),
		cmMsg("m2", "u_me", "Me Myself", "second", "2026-01-01T10:01:00.000Z"),
		cmMsg("m3", "a_bot", "Helper Bot", "third", "2026-01-01T10:02:00.000Z"),
	}
	c.msgs["p_study"] = []obj{
		cmMsg("s1", "u_bob", "Bob Builder", "notes", "2026-01-02T10:00:00.000Z"),
		cmMsg("s2", "u_me", "Me Myself", "more notes", "2026-01-02T10:05:00.000Z"),
	}
	c.msgs["p_dm"] = []obj{cmMsg("d1", "u_me", "Me Myself", "to the bot", "2026-01-03T10:00:00.000Z"), cmMsg("d2", "a_bot", "Helper Bot", "reply", "2026-01-03T10:01:00.000Z")}
	c.msgs["p_foreign"] = []obj{cmMsg("x1", "a_x", "X", "chatter", "2026-01-04T10:00:00.000Z")}
	return c
}

func (c *cmy) serve(t *testing.T) *httptest.Server {
	srv := httptest.NewServer(http.HandlerFunc(c.handle))
	t.Cleanup(srv.Close)
	return srv
}

func (c *cmy) seen(sub string) int {
	c.mu.Lock()
	defer c.mu.Unlock()
	n := 0
	for _, r := range c.reqs {
		if strings.Contains(r, sub) {
			n++
		}
	}
	return n
}

func cmJSON(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(v)
}

func (c *cmy) handle(w http.ResponseWriter, r *http.Request) {
	n := c.cur.Add(1)
	defer c.cur.Add(-1)
	if m := c.max.Load(); n > m {
		c.max.CompareAndSwap(m, n)
	}
	c.mu.Lock()
	c.reqs = append(c.reqs, r.Method+" "+r.URL.RequestURI()+" auth="+r.Header.Get("Authorization"))
	c.calls[r.URL.Path]++
	call := c.calls[r.URL.Path]
	c.mu.Unlock()
	last := r.URL.Path[strings.LastIndex(r.URL.Path, "/")+1:]
	if c.failMsg != nil && strings.HasPrefix(r.URL.Path, "/api/messages/") {
		if code, ra := c.failMsg(last, call); code != 0 {
			if ra != "" {
				w.Header().Set("Retry-After", ra)
			}
			w.WriteHeader(code)
			return
		}
	}
	switch {
	case r.URL.Path == "/api/auth/user":
		cmJSON(w, c.me)
	case r.URL.Path == "/api/pods":
		cmJSON(w, c.pods)
	case strings.HasPrefix(r.URL.Path, "/api/pods/"):
		for _, p := range c.pods {
			if p["_id"] == last {
				cmJSON(w, p)
				return
			}
		}
		w.WriteHeader(404)
	case strings.HasPrefix(r.URL.Path, "/api/messages/"):
		cmJSON(w, c.page(last, r.URL.Query().Get("before")))
	case r.URL.Path == "/api/grants":
		cmJSON(w, obj{"grants": c.grants})
	case strings.HasPrefix(r.URL.Path, "/api/integrations/"):
		cmJSON(w, c.integ[last])
	default:
		w.WriteHeader(404)
	}
}

func (c *cmy) page(pod, before string) []obj {
	all := append([]obj(nil), c.msgs[pod]...)
	sort.SliceStable(all, func(i, j int) bool { return all[i]["createdAt"].(string) < all[j]["createdAt"].(string) })
	var kept []obj
	for _, m := range all {
		if before == "" || m["createdAt"].(string) < before {
			kept = append(kept, m)
		}
	}
	if len(kept) > 50 {
		kept = kept[len(kept)-50:]
	}
	return kept
}
