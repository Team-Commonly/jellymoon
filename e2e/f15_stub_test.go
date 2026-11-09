package e2e

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"sync"
)

// ghStub is a stand-in for GitHub's OAuth web flow and GET /user, at the GitHub Enterprise layout the server uses for an
// override base: <base>/login/oauth/authorize, <base>/login/oauth/access_token and <base>/api/v3/user. The suite never
// calls the real GitHub (docs/development/e2e-contract.md, "How the suite starts the server").
type ghStub struct {
	srv     *httptest.Server
	secret  string
	mu      sync.Mutex
	login   string // who the next authorize signs in as
	id      int64
	revoked bool              // GET /user answers 401
	codes   map[string]string // OAuth code -> GitHub token
	tokens  map[string]bool   // every GitHub token handed out
	users   map[string]ghUser // GitHub token -> user
}

type ghUser struct {
	ID    int64  `json:"id"`
	Login string `json:"login"`
}

func randHex(n int) string {
	b := make([]byte, n)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}

// gh is the suite's one stub GitHub. It starts before any test, and its URL and OAuth client go into the process
// environment, which every server the suite starts inherits; only TestF15 signs anyone in.
var gh = newGHStub()

func newGHStub() *ghStub {
	g := &ghStub{secret: randHex(20), login: "octo-ana", id: 4100001, codes: map[string]string{}, tokens: map[string]bool{}, users: map[string]ghUser{}}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /login/oauth/authorize", g.authorize)
	mux.HandleFunc("POST /login/oauth/access_token", g.accessToken)
	mux.HandleFunc("GET /api/v3/user", g.user)
	g.srv = httptest.NewServer(mux)
	for _, kv := range [][2]string{{"GITHUB_URL", g.srv.URL}, {"GITHUB_CLIENT_ID", "jm-e2e-client"}, {"GITHUB_CLIENT_SECRET", g.secret}} {
		_ = os.Setenv(kv[0], kv[1])
	}
	return g
}

func (g *ghStub) as(login string, id int64, revoked bool) {
	g.mu.Lock()
	defer g.mu.Unlock()
	g.login, g.id, g.revoked = login, id, revoked
}

// authorize approves at once, as a person who clicks Authorize would, and sends the browser back with a code and the state.
func (g *ghStub) authorize(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	back, err := url.Parse(q.Get("redirect_uri"))
	if q.Get("client_id") != "jm-e2e-client" || err != nil || q.Get("state") == "" {
		http.Error(w, "bad authorize request", 400)
		return
	}
	g.mu.Lock()
	code, tok := randHex(10), "gho_stub"+randHex(18)
	g.codes[code], g.tokens[tok], g.users[tok] = tok, true, ghUser{ID: g.id, Login: g.login}
	g.mu.Unlock()
	v := back.Query()
	v.Set("code", code)
	v.Set("state", q.Get("state"))
	back.RawQuery = v.Encode()
	http.Redirect(w, r, back.String(), http.StatusFound)
}

// accessToken exchanges a code once, for the client that holds the secret.
func (g *ghStub) accessToken(w http.ResponseWriter, r *http.Request) {
	code := r.FormValue("code")
	g.mu.Lock()
	tok, ok := g.codes[code]
	delete(g.codes, code)
	g.mu.Unlock()
	ok = ok && r.FormValue("client_id") == "jm-e2e-client" && r.FormValue("client_secret") == g.secret
	w.Header().Set("Content-Type", "application/json")
	if !ok {
		_ = json.NewEncoder(w).Encode(map[string]string{"error": "bad_verification_code"})
		return
	}
	_ = json.NewEncoder(w).Encode(map[string]string{"access_token": tok, "token_type": "bearer", "scope": ""})
}

func (g *ghStub) user(w http.ResponseWriter, r *http.Request) {
	g.mu.Lock()
	u, ok := g.users[stripBearer(r.Header.Get("Authorization"))]
	revoked := g.revoked
	g.mu.Unlock()
	if !ok || revoked {
		w.WriteHeader(401)
		_, _ = fmt.Fprint(w, `{"message":"Bad credentials"}`)
		return
	}
	_ = json.NewEncoder(w).Encode(u)
}

func stripBearer(h string) string {
	for _, p := range []string{"Bearer ", "bearer ", "token "} {
		if len(h) > len(p) && h[:len(p)] == p {
			return h[len(p):]
		}
	}
	return ""
}

// issued is every GitHub token the stub handed out.
func (g *ghStub) issued() []string {
	g.mu.Lock()
	defer g.mu.Unlock()
	out := make([]string, 0, len(g.tokens))
	for k := range g.tokens {
		out = append(out, k)
	}
	return out
}
