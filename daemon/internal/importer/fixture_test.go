package importer

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

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

type obj = map[string]any

// fx is a fixture Commonly instance: the shapes of the real routes, and a record of every request.
type fx struct {
	t       *testing.T
	srv     *httptest.Server
	me      obj
	pods    []obj
	msgs    map[string][]obj
	grants  []obj
	integ   map[string][]obj
	mu      sync.Mutex
	reqs    []string
	cur     atomic.Int32
	maxCur  atomic.Int32
	failMsg func(podID string, call int) int // a status to answer with instead, 0 for none
	calls   map[string]int
}

func member(id, name string, bot bool) obj {
	return obj{"_id": id, "username": name, "isBot": bot, "profilePicture": "https://img.example/" + id + ".png"}
}

func message(id, userID, user, text, at string) obj {
	return obj{"id": id, "_id": id, "user_id": userID, "username": user, "content": text, "messageType": "text", "createdAt": at, "created_at": at}
}

func newFx(t *testing.T) *fx {
	me := member("u_me", "Me Myself", false)
	me["role"] = "user"
	me["email"] = "me@example.test"
	bob, bot := member("u_bob", "Bob Builder", false), member("a_bot", "Helper Bot", true)
	bob["email"] = "bob@example.test"
	f := &fx{t: t, me: me, msgs: map[string][]obj{}, integ: map[string][]obj{}, calls: map[string]int{}}
	f.pods = []obj{
		{"_id": "p_team", "name": "Team room", "type": "team", "members": []obj{me, bob, bot}},
		{"_id": "p_dm", "name": "Me and Helper", "type": "agent-room", "members": []obj{me, bot}},
		{"_id": "p_admin", "name": "Admin room", "type": "agent-admin", "members": []obj{me}},
		{"_id": "p_foreign", "name": "Others DM", "type": "agent-dm", "members": []obj{member("a_x", "X", true), member("a_y", "Y", true)}},
		{"_id": "p_study", "name": "Study", "type": "study", "members": []obj{me, bob}},
	}
	f.msgs["p_team"] = []obj{
		message("m1", "u_bob", "Bob Builder", "first", "2026-01-01T10:00:00.000Z"),
		message("m2", "u_me", "Me Myself", "second\nmore lines", "2026-01-01T10:01:00.000Z"),
		message("m3", "a_bot", "Helper Bot", "third", "2026-01-01T10:02:00.000Z"),
	}
	f.msgs["p_study"] = []obj{
		message("s1", "u_bob", "Bob Builder", "notes", "2026-01-02T10:00:00.000Z"),
		message("s2", "u_me", "Me Myself", "more notes", "2026-01-02T10:05:00.000Z"),
	}
	f.msgs["p_dm"] = []obj{message("d1", "u_me", "Me Myself", "private to the bot", "2026-01-03T10:00:00.000Z"), message("d2", "a_bot", "Helper Bot", "reply", "2026-01-03T10:01:00.000Z")}
	f.msgs["p_foreign"] = []obj{message("x1", "a_x", "X", "secret chatter", "2026-01-04T10:00:00.000Z")}
	return f
}

func (f *fx) server() *httptest.Server {
	if f.srv == nil {
		f.srv = httptest.NewServer(http.HandlerFunc(f.serve))
		f.t.Cleanup(f.srv.Close)
	}
	return f.srv
}

func (f *fx) count(sub string) int {
	f.mu.Lock()
	defer f.mu.Unlock()
	n := 0
	for _, r := range f.reqs {
		if strings.Contains(r, sub) {
			n++
		}
	}
	return n
}

func (f *fx) serve(w http.ResponseWriter, r *http.Request) {
	now := f.cur.Add(1)
	defer f.cur.Add(-1)
	for {
		m := f.maxCur.Load()
		if now <= m || f.maxCur.CompareAndSwap(m, now) {
			break
		}
	}
	f.mu.Lock()
	f.reqs = append(f.reqs, r.Method+" "+r.URL.RequestURI())
	f.calls[r.URL.Path]++
	call := f.calls[r.URL.Path]
	f.mu.Unlock()
	parts := strings.Split(strings.Trim(r.URL.Path, "/"), "/")
	podID := parts[len(parts)-1]
	if f.failMsg != nil && strings.HasPrefix(r.URL.Path, "/api/messages/") {
		if code := f.failMsg(podID, call); code != 0 {
			w.WriteHeader(code)
			return
		}
	}
	switch {
	case r.URL.Path == "/api/auth/user":
		reply(w, f.me)
	case r.URL.Path == "/api/pods":
		reply(w, f.pods)
	case strings.HasPrefix(r.URL.Path, "/api/pods/"):
		for _, p := range f.pods {
			if p["_id"] == podID {
				reply(w, p)
				return
			}
		}
		w.WriteHeader(404)
	case strings.HasPrefix(r.URL.Path, "/api/messages/"):
		reply(w, f.page(podID, r.URL.Query().Get("before"), r.URL.Query().Get("limit")))
	case r.URL.Path == "/api/grants":
		reply(w, obj{"grants": f.grants})
	case strings.HasPrefix(r.URL.Path, "/api/integrations/"):
		reply(w, f.integ[podID])
	default:
		w.WriteHeader(404)
	}
}

// page is the real route's page: the newest `limit` messages created strictly before `before`, oldest first.
func (f *fx) page(podID, before, limit string) []obj {
	n, _ := strconv.Atoi(limit)
	all := append([]obj(nil), f.msgs[podID]...)
	sort.SliceStable(all, func(i, j int) bool { return all[i]["createdAt"].(string) < all[j]["createdAt"].(string) })
	var kept []obj
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

func reply(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(v)
}

// fakeSink is Jellymoon: it records what is written, and can lose the acknowledgement of one post (it lands, the caller sees an error).
type fakeSink struct {
	mu        sync.Mutex
	pods      map[string]string // name -> id
	posts     map[string][]protocol.PostInput
	writes    int
	failAfter int // posts that succeed before every later post fails; 0 means never fail
	lostAck   int // the nth post lands but returns an error; 0 means never
}

func newSink() *fakeSink {
	return &fakeSink{pods: map[string]string{}, posts: map[string][]protocol.PostInput{}}
}

func (s *fakeSink) CreatePod(name string) (string, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.writes++
	id := "pod_" + strconv.Itoa(len(s.pods)+1)
	s.pods[name] = id
	return id, nil
}

func (s *fakeSink) Post(podID string, in protocol.PostInput) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	total := 0
	for _, p := range s.posts {
		total += len(p)
	}
	if s.failAfter > 0 && total >= s.failAfter {
		return http.ErrHandlerTimeout
	}
	s.writes++
	s.posts[podID] = append(s.posts[podID], in)
	if s.lostAck > 0 && total+1 == s.lostAck {
		return http.ErrHandlerTimeout
	}
	return nil
}

func (s *fakeSink) Sources(podID string) (map[string]bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := map[string]bool{}
	for _, p := range s.posts[podID] {
		if p.Body != nil {
			out[markerOf(*p.Body)] = true
		}
	}
	return out, nil
}

// bodies lists every posted body of every pod, in posting order per pod.
func (s *fakeSink) bodies() []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	var out []string
	for _, id := range s.sortedPods() {
		for _, p := range s.posts[id] {
			out = append(out, *p.Body)
		}
	}
	return out
}

func (s *fakeSink) sortedPods() []string {
	var ids []string
	for id := range s.posts {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return ids
}
