//go:build !linux && !darwin

package daemon

import (
	"errors"
	"net"
)

// peerUID refuses: without a kernel-reported uid, no peer is trusted.
func peerUID(*net.UnixConn) (uint32, error) {
	return 0, errors.New("this OS has no peer credential call the daemon supports")
}
