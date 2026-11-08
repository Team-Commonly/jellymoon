package daemon

import (
	"encoding/binary"
	"errors"
)

// parseXucred reads the uid out of a darwin xucred { u_int cr_version; uid_t cr_uid; ... }. The length getsockopt reported is
// not checked yet.
func parseXucred(buf []byte, n int) (uint32, error) {
	if binary.NativeEndian.Uint32(buf[0:4]) != 0 {
		return 0, errors.New("unknown xucred version")
	}
	return binary.NativeEndian.Uint32(buf[4:8]), nil
}
