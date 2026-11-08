package daemon

import (
	"encoding/binary"
	"errors"
)

// parseXucred reads the uid out of a darwin xucred { u_int cr_version; uid_t cr_uid; ... }. n is the length getsockopt
// reported: the uid counts only when the kernel filled the version and uid fields, so a short result is refused and never
// reads as uid 0 from a zeroed buffer.
func parseXucred(buf []byte, n int) (uint32, error) {
	if n < 8 || n > len(buf) {
		return 0, errors.New("the peer credential came back short")
	}
	if binary.NativeEndian.Uint32(buf[0:4]) != 0 {
		return 0, errors.New("unknown xucred version")
	}
	return binary.NativeEndian.Uint32(buf[4:8]), nil
}
