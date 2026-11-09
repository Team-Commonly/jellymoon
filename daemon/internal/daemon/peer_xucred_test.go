package daemon

import (
	"encoding/binary"
	"testing"
)

func xucredOf(version, uid uint32, size int) []byte {
	b := make([]byte, size)
	binary.NativeEndian.PutUint32(b[0:4], version)
	binary.NativeEndian.PutUint32(b[4:8], uid)
	return b
}

// A short getsockopt result must not read as uid 0 from a zeroed buffer: the uid counts only when the kernel reported it.
func TestParseXucred(t *testing.T) {
	cases := []struct {
		name    string
		buf     []byte
		n       int
		want    uint32
		wantErr bool
	}{
		{"full", xucredOf(0, 501, 76), 76, 501, false},
		{"exactly the uid field", xucredOf(0, 501, 76), 8, 501, false},
		{"zero length", xucredOf(0, 0, 76), 0, 0, true},
		{"version only", xucredOf(0, 0, 76), 4, 0, true},
		{"seven bytes", xucredOf(0, 0, 76), 7, 0, true},
		{"negative length", xucredOf(0, 0, 76), -1, 0, true},
		{"length past the buffer", xucredOf(0, 501, 8), 76, 0, true},
		{"unknown version", xucredOf(1, 501, 76), 76, 0, true},
	}
	for _, c := range cases {
		got, err := parseXucred(c.buf, c.n)
		if (err != nil) != c.wantErr || (err == nil && got != c.want) {
			t.Errorf("%s: parseXucred(n=%d) = %d, %v; want %d, error=%v", c.name, c.n, got, err, c.want, c.wantErr)
		}
	}
}
