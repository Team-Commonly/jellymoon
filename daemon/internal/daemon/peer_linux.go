//go:build linux

package daemon

import (
	"net"
	"syscall"
)

// peerUID is the uid of the process that connected, from SO_PEERCRED.
func peerUID(c *net.UnixConn) (uint32, error) {
	rc, err := c.SyscallConn()
	if err != nil {
		return 0, err
	}
	var uid uint32
	var serr error
	if err := rc.Control(func(fd uintptr) {
		var u *syscall.Ucred
		if u, serr = syscall.GetsockoptUcred(int(fd), syscall.SOL_SOCKET, syscall.SO_PEERCRED); serr == nil {
			uid = u.Uid
		}
	}); err != nil {
		return 0, err
	}
	return uid, serr
}
