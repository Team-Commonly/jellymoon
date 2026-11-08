package daemon

import (
	"errors"
	"net"
)

// sameUser is not checking yet.
func sameUser(peer, self uint32) bool { return true }

// peerUID is not reading credentials yet.
func peerUID(c net.Conn) (uint32, error) { return 0, errors.New("not implemented") }
