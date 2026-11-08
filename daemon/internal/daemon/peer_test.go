package daemon

import (
	"bufio"
	"io"
	"net"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
)

// The comparison is a pure function so the other-user path can be tested without a second account: a real other-uid
// connection needs root to fake.
func TestSameUser(t *testing.T) {
	cases := []struct {
		peer, self uint32
		want       bool
	}{{501, 501, true}, {0, 0, true}, {501, 502, false}, {0, 501, false}, {501, 0, false}}
	for _, c := range cases {
		if got := sameUser(c.peer, c.self); got != c.want {
			t.Errorf("sameUser(peer %d, self %d) = %v, want %v", c.peer, c.self, got, c.want)
		}
	}
}

// listenShort makes a unix socket under /tmp: a test's own temp dir can pass the 104 bytes sun_path allows on macOS.
func listenShort(t *testing.T) (net.Listener, string) {
	t.Helper()
	dir, err := os.MkdirTemp("/tmp", "jmp")
	if err != nil {
		t.Fatal(err)
	}
	p := filepath.Join(dir, "s")
	l, err := net.Listen("unix", p)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { l.Close(); _ = os.Remove(p); _ = os.Remove(dir) })
	return l, p
}

// A connection from this process carries this process's uid, read by the OS rather than claimed by the peer.
func TestPeerUIDIsTheCallersOwn(t *testing.T) {
	l, p := listenShort(t)
	accepted := make(chan net.Conn, 1)
	go func() { c, _ := l.Accept(); accepted <- c }()
	cl, err := net.Dial("unix", p)
	if err != nil {
		t.Fatal(err)
	}
	defer cl.Close()
	srv := <-accepted
	defer srv.Close()
	uid, err := peerUID(srv)
	if err != nil {
		t.Fatalf("peerUID: %v", err)
	}
	if uid != uint32(os.Geteuid()) {
		t.Errorf("peer uid %d, want %d", uid, os.Geteuid())
	}
}

// ask sends one line to the control socket and returns what comes back before the close.
func ask(t *testing.T, path string) string {
	t.Helper()
	c, err := net.Dial("unix", path)
	if err != nil {
		t.Fatal(err)
	}
	defer c.Close()
	_ = c.SetDeadline(time.Now().Add(5 * time.Second))
	_, _ = io.WriteString(c, "not json\n")
	line, _ := bufio.NewReader(c).ReadString('\n')
	return line
}

// The accept loop answers a connection from the daemon's own uid, and closes one from any other with no reply. The other
// uid is stood in for by telling the loop it runs as someone else.
func TestServeClosesAConnectionFromAnotherUserWithNoReply(t *testing.T) {
	d := New(nil, config.Store{Dir: t.TempDir()}, config.Computer{}, nil)
	own, ownPath := listenShort(t)
	go d.serveAs(own, uint32(os.Geteuid()))
	if got := ask(t, ownPath); got == "" {
		t.Error("a same-uid connection got no reply")
	}
	other, otherPath := listenShort(t)
	go d.serveAs(other, uint32(os.Geteuid())+1)
	if got := ask(t, otherPath); got != "" {
		t.Errorf("a connection from another uid got a reply: %q", got)
	}
}
