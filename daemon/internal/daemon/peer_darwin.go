//go:build darwin

package daemon

import (
	"encoding/binary"
	"errors"
	"net"
	"syscall"
	"unsafe"
)

// LOCAL_PEERCRED on SOL_LOCAL is what getpeereid(3) reads: an xucred { u_int cr_version; uid_t cr_uid; ... }, 76 bytes. The
// syscall package has no wrapper for it and the module takes no dependencies, so the call is made directly.
const (
	solLocal       = 0
	localPeercred  = 1
	xucredSize     = 76
	xucredVersion0 = 0
)

// peerUID is the uid of the process that connected.
func peerUID(c *net.UnixConn) (uint32, error) {
	rc, err := c.SyscallConn()
	if err != nil {
		return 0, err
	}
	var uid uint32
	var serr error
	if err := rc.Control(func(fd uintptr) {
		var cred [xucredSize]byte
		n := uint32(len(cred))
		_, _, e := syscall.Syscall6(syscall.SYS_GETSOCKOPT, fd, solLocal, localPeercred, uintptr(unsafe.Pointer(&cred[0])), uintptr(unsafe.Pointer(&n)), 0)
		switch {
		case e != 0:
			serr = e
		case binary.NativeEndian.Uint32(cred[0:4]) != xucredVersion0:
			serr = errors.New("unknown xucred version")
		default:
			uid = binary.NativeEndian.Uint32(cred[4:8])
		}
	}); err != nil {
		return 0, err
	}
	return uid, serr
}
