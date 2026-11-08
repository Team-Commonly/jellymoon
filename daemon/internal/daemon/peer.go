package daemon

import "net"

// The control socket is 0600 inside a 0700 dir, and that is the first line. This is the second: the kernel's record of
// who is on the other end of each accepted connection, which no peer can claim for itself. peerUID is per OS (peer_linux.go
// SO_PEERCRED, peer_darwin.go LOCAL_PEERCRED, the call getpeereid wraps); an OS with neither refuses every peer.

// sameUser is the whole decision, kept pure so the other-user case can be tested without a second account.
func sameUser(peer, self uint32) bool { return peer == self }

// permit reports whether the connection comes from the uid the daemon runs as. A peer whose uid cannot be read is refused.
func permit(c net.Conn, self uint32) bool {
	uc, ok := c.(*net.UnixConn)
	if !ok {
		return false
	}
	uid, err := peerUID(uc)
	return err == nil && sameUser(uid, self)
}
