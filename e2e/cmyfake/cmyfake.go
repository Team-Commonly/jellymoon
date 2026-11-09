// Package cmyfake is a fixture Commonly instance with the shapes of the real routes (the CMY backend), shared by the importer's unit
// tests and the e2e test. The pod listing populates members' botMetadata and GET /api/pods/:id does not, as the real routes do.
package cmyfake

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sort"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
)

// Obj is a JSON object.
type Obj = map[string]any

// Server is the fixture. Fill the exported fields, then Serve it.
type Server struct {
	Me      Obj
	Pods    []Obj
	Msgs    map[string][]Obj
	Grants  []Obj
	Integ   map[string][]Obj
	FailMsg func(pod string, call int) (status int, retryAfter string) // answers a messages call with this status instead; 0 for none
	mu      sync.Mutex
	reqs    []string
	calls   map[string]int
	cur, hi atomic.Int32
	srv     *httptest.Server
}

// Member is a populated pod member. A bot also carries botMetadata, which only the listing returns.
func Member(id, name string, bot bool) Obj {
	m := Obj{"_id": id, "username": name, "isBot": bot, "profilePicture": "https://img.example/" + id + ".png"}
	if bot {
		m["botMetadata"] = Obj{"displayName": name + " (agent)", "instanceId": "inst-" + id}
	}
	return m
}

// Msg is one message as the route answers it.
func Msg(id, uid, name, text, at string) Obj {
	return Obj{"id": id, "user_id": uid, "username": name, "content": text, "messageType": "text", "createdAt": at}
}

// New is the usual world: a person, a person they talk to, an agent, and five pods.
func New() *Server {
	me := Member("u_me", "Me Myself", false)
	me["role"], me["email"] = "user", "me@example.com"
	bob, bot := Member("u_bob", "Bob Builder", false), Member("a_bot", "Helper Bot", true)
	bob["email"] = "bob@example.com"
	s := &Server{Me: me, Msgs: map[string][]Obj{}, Integ: map[string][]Obj{}, calls: map[string]int{}}
	s.Pods = []Obj{
		{"_id": "p_team", "name": "Team room", "type": "team", "members": []Obj{me, bob, bot}},
		{"_id": "p_dm", "name": "Me and Helper", "type": "agent-room", "members": []Obj{me, bot}},
		{"_id": "p_admin", "name": "Admin room", "type": "agent-admin", "members": []Obj{me}},
		{"_id": "p_foreign", "name": "Others DM", "type": "agent-dm", "members": []Obj{Member("a_x", "X", true), Member("a_y", "Y", true)}},
		{"_id": "p_study", "name": "Study", "type": "study", "members": []Obj{me, bob}},
	}
	s.Msgs["p_team"] = []Obj{
		Msg("m1", "u_bob", "Bob Builder", "first", "2026-01-01T10:00:00.000Z"),
		Msg("m2", "u_me", "Me Myself", "second\nmore lines", "2026-01-01T10:01:00.000Z"),
		Msg("m3", "a_bot", "Helper Bot", "third", "2026-01-01T10:02:00.000Z"),
	}
	s.Msgs["p_study"] = []Obj{
		Msg("s1", "u_bob", "Bob Builder", "notes", "2026-01-02T10:00:00.000Z"),
		Msg("s2", "u_me", "Me Myself", "more notes", "2026-01-02T10:05:00.000Z"),
	}
	s.Msgs["p_dm"] = []Obj{Msg("d1", "u_me", "Me Myself", "to the bot", "2026-01-03T10:00:00.000Z"), Msg("d2", "a_bot", "Helper Bot", "reply", "2026-01-03T10:01:00.000Z")}
	s.Msgs["p_foreign"] = []Obj{Msg("x1", "a_x", "X", "chatter", "2026-01-04T10:00:00.000Z")}
	return s
}

// Serve starts the fixture once (a second call returns the same server); it stops with the test.
func (s *Server) Serve(t testing.TB) *httptest.Server {
	if s.srv == nil {
		s.srv = httptest.NewServer(http.HandlerFunc(s.handle))
		t.Cleanup(s.srv.Close)
	}
	return s.srv
}

// Seen counts the requests ("METHOD uri auth=header") that contain sub.
func (s *Server) Seen(sub string) int {
	n := 0
	for _, r := range s.Requests() {
		if strings.Contains(r, sub) {
			n++
		}
	}
	return n
}

// Requests is every request so far.
func (s *Server) Requests() []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]string(nil), s.reqs...)
}

// MaxInFlight is the most requests the fixture ever had open at once.
func (s *Server) MaxInFlight() int { return int(s.hi.Load()) }

func send(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(v)
}

func (s *Server) handle(w http.ResponseWriter, r *http.Request) {
	n := s.cur.Add(1)
	defer s.cur.Add(-1)
	for m := s.hi.Load(); n > m && !s.hi.CompareAndSwap(m, n); m = s.hi.Load() {
	}
	s.mu.Lock()
	s.reqs = append(s.reqs, r.Method+" "+r.URL.RequestURI()+" auth="+r.Header.Get("Authorization"))
	s.calls[r.URL.Path]++
	call := s.calls[r.URL.Path]
	s.mu.Unlock()
	last := r.URL.Path[strings.LastIndex(r.URL.Path, "/")+1:]
	if s.FailMsg != nil && strings.HasPrefix(r.URL.Path, "/api/messages/") {
		if code, retryAfter := s.FailMsg(last, call); code != 0 {
			if retryAfter != "" {
				w.Header().Set("Retry-After", retryAfter)
			}
			w.WriteHeader(code)
			return
		}
	}
	switch {
	case r.URL.Path == "/api/auth/user":
		send(w, s.Me)
	case r.URL.Path == "/api/pods":
		send(w, s.Pods)
	case strings.HasPrefix(r.URL.Path, "/api/pods/"):
		for _, p := range s.Pods {
			if p["_id"] == last {
				send(w, withoutBotMetadata(p))
				return
			}
		}
		w.WriteHeader(404)
	case strings.HasPrefix(r.URL.Path, "/api/messages/"):
		send(w, s.page(last, r.URL.Query().Get("before"), r.URL.Query().Get("limit")))
	case r.URL.Path == "/api/grants":
		send(w, Obj{"grants": s.Grants})
	case strings.HasPrefix(r.URL.Path, "/api/integrations/"):
		send(w, s.Integ[last])
	default:
		w.WriteHeader(404)
	}
}

// withoutBotMetadata is getPodById's answer: its populate is 'username profilePicture isBot', so botMetadata is absent.
func withoutBotMetadata(p Obj) Obj {
	out := Obj{}
	for k, v := range p {
		out[k] = v
	}
	var ms []Obj
	for _, m := range p["members"].([]Obj) {
		c := Obj{}
		for k, v := range m {
			if k != "botMetadata" {
				c[k] = v
			}
		}
		ms = append(ms, c)
	}
	out["members"] = ms
	return out
}

// page is the real route's page: the newest `limit` messages created strictly before `before`, oldest first.
func (s *Server) page(pod, before, limit string) []Obj {
	n, _ := strconv.Atoi(limit)
	all := append([]Obj(nil), s.Msgs[pod]...)
	sort.SliceStable(all, func(i, j int) bool { return all[i]["createdAt"].(string) < all[j]["createdAt"].(string) })
	var kept []Obj
	for _, m := range all {
		if before == "" || m["createdAt"].(string) < before {
			kept = append(kept, m)
		}
	}
	if n > 0 && len(kept) > n {
		kept = kept[len(kept)-n:]
	}
	return kept
}
