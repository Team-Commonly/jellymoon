package e2e

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

// These tests need no server: they check the switches that make the suite run against a deployed URL.

func TestRemoteSetupChecks(t *testing.T) {
	if err := checkRemote("", "https://x.example", "b"); err != nil {
		t.Fatalf("a URL with no server command and a bearer is the remote mode: %v", err)
	}
	for name, c := range map[string][3]string{
		"a URL and a server command": {"cmd", "https://x.example", "b"},
		"a URL with no bearer":       {"", "https://x.example", ""},
		"plain http to a remote":     {"", "http://x.example", "b"},
	} {
		if checkRemote(c[0], c[1], c[2]) == nil {
			t.Errorf("%s was accepted", name)
		}
	}
}

func TestRemoteClientSendsBearer(t *testing.T) {
	seen := map[string]string{}
	ts := httptest.NewServer(http.HandlerFunc(func(_ http.ResponseWriter, r *http.Request) { seen[r.URL.Path] = r.Header.Get("Authorization") }))
	defer ts.Close()
	c := newClient(t, ts.URL)
	c.testBearer = "sekrit"
	c.do("POST", "/__test/session", "", nil, nil)
	c.do("GET", "/v1/pods", "", nil, nil)
	c.do("GET", "/v1/me", "jmu_tok", nil, nil)
	if seen["/__test/session"] != "Bearer sekrit" || seen["/v1/pods"] != "" || seen["/v1/me"] != "Bearer jmu_tok" {
		t.Fatalf("Authorization by path: %v", seen)
	}
}

func TestRemoteSkipsFakeClockTests(t *testing.T) {
	oldCmd, oldReal := serverCmd, realClock
	defer func() { serverCmd, realClock = oldCmd, oldReal }()
	serverCmd, realClock = "a server command", true // need() passes, so only the clock can skip
	skippedBy := func(f func(*testing.T)) (skipped bool) {
		t.Run("probe", func(it *testing.T) {
			defer func() { skipped = it.Skipped() }()
			f(it)
		})
		return
	}
	if !skippedBy(needFake) {
		t.Error("needFake did not skip in real-clock mode")
	}
	if skippedBy(needClockFree) {
		t.Error("needClockFree skipped in real-clock mode")
	}
}

func TestRemoteZeroTestsFail(t *testing.T) {
	if judgeRan(0, 0) != 1 || judgeRan(0, 3) != 0 || judgeRan(1, 0) != 1 {
		t.Fatal("judgeRan must fail a green run that ran nothing and leave other runs alone")
	}
}

func TestRemoteServerIsNeverStarted(t *testing.T) {
	old := remoteURL
	defer func() { remoteURL = old }()
	remoteURL = "https://x.example"
	s := startServer(t)
	s.kill()
	if s.cmd != nil || s.base() != "https://x.example" {
		t.Fatalf("remote server has a process or the wrong base: %+v", s)
	}
}
