package e2e

import (
	"bufio"
	"crypto/rand"
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"sync"
	"time"
)

// A minimal RFC 6455 client (text frames, ping/pong, close). No dependency.

func wsDial(addr, path, token string) (net.Conn, *bufio.Reader, error) {
	c, err := net.DialTimeout("tcp", addr, 5*time.Second)
	if err != nil {
		return nil, nil, err
	}
	raw := make([]byte, 16)
	_, _ = rand.Read(raw)
	key := base64.StdEncoding.EncodeToString(raw)
	req, _ := http.NewRequest("GET", "http://"+addr+path, nil)
	for k, v := range map[string]string{"Upgrade": "websocket", "Connection": "Upgrade", "Sec-WebSocket-Key": key,
		"Sec-WebSocket-Version": "13", "Authorization": "Bearer " + token} {
		req.Header.Set(k, v)
	}
	_ = c.SetDeadline(time.Now().Add(5 * time.Second))
	br := bufio.NewReader(c)
	var resp *http.Response
	if err = req.Write(c); err == nil {
		resp, err = http.ReadResponse(br, req)
	}
	if err == nil && resp.StatusCode != 101 {
		b, _ := io.ReadAll(io.LimitReader(resp.Body, 300))
		err = fmt.Errorf("upgrade refused: HTTP %d %s", resp.StatusCode, b)
	}
	sum := sha1.Sum([]byte(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
	if err == nil && resp.Header.Get("Sec-WebSocket-Accept") != base64.StdEncoding.EncodeToString(sum[:]) {
		err = fmt.Errorf("bad Sec-WebSocket-Accept")
	}
	if err != nil {
		c.Close()
		return nil, nil, err
	}
	_ = c.SetDeadline(time.Time{})
	return c, br, nil
}

func writeFrame(c net.Conn, op byte, p []byte) error {
	h := []byte{0x80 | op}
	switch n := len(p); {
	case n < 126:
		h = append(h, 0x80|byte(n))
	case n < 65536:
		h = append(h, 0x80|126, byte(n>>8), byte(n))
	default:
		h = binary.BigEndian.AppendUint64(append(h, 0x80|127), uint64(n))
	}
	mask := make([]byte, 4)
	_, _ = rand.Read(mask)
	out := append(h, mask...)
	for i, b := range p {
		out = append(out, b^mask[i%4])
	}
	_, err := c.Write(out)
	return err
}

func readFrame(br *bufio.Reader) (fin bool, op byte, p []byte, err error) {
	var h [2]byte
	if _, err = io.ReadFull(br, h[:]); err != nil {
		return
	}
	fin, op = h[0]&0x80 != 0, h[0]&0x0f
	n := uint64(h[1] & 0x7f)
	if n >= 126 {
		ext := make([]byte, map[uint64]int{126: 2, 127: 8}[n])
		if _, err = io.ReadFull(br, ext); err != nil {
			return
		}
		n = 0
		for _, b := range ext {
			n = n<<8 | uint64(b)
		}
	}
	var mask [4]byte
	if h[1]&0x80 != 0 {
		if _, err = io.ReadFull(br, mask[:]); err != nil {
			return
		}
	}
	p = make([]byte, n)
	if _, err = io.ReadFull(br, p); err == nil {
		for i := range p {
			p[i] ^= mask[i%4] * byte(h[1]>>7)
		}
	}
	return
}

// Frame is the Envelope. recv is when the suite read it, in unix ms.
type Frame struct {
	V     int             `json:"v"`
	Type  string          `json:"type"`
	Seq   int64           `json:"seq"`
	PodID string          `json:"podId"`
	Ts    string          `json:"ts"`
	Data  json.RawMessage `json:"data"`
	recv  int64
}

func (f Frame) dec(v any) { _ = json.Unmarshal(f.Data, v) }

type sock struct {
	name, pod string
	conn      net.Conn
	wmu, mu   sync.Mutex
	frames    []Frame
	dead      bool
	ch        chan struct{}
}

func newSock(name, pod string, c net.Conn, br *bufio.Reader) *sock {
	s := &sock{name: name, pod: pod, conn: c, ch: make(chan struct{})}
	go s.read(br)
	return s
}

// push appends f (if any) and wakes every waiter; dead marks the socket closed.
func (s *sock) push(f *Frame, dead bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if f != nil {
		s.frames = append(s.frames, *f)
	}
	s.dead = s.dead || dead
	close(s.ch)
	s.ch = make(chan struct{})
}

func (s *sock) read(br *bufio.Reader) {
	for {
		_, op, p, err := readFrame(br)
		if err != nil || op == 8 {
			s.push(nil, true)
			return
		}
		if op == 9 {
			s.wmu.Lock()
			_ = writeFrame(s.conn, 10, p)
			s.wmu.Unlock()
		}
		var f Frame // the pod sends whole text frames; a fragmented or malformed one is dropped
		if op == 1 && json.Unmarshal(p, &f) == nil && f.V == 1 && f.Type != "" {
			f.recv = time.Now().UnixMilli()
			s.push(&f, false)
		}
	}
}

func (s *sock) send(typ string, data any) {
	b, _ := json.Marshal(map[string]any{"v": 1, "type": typ, "podId": s.pod, "data": data})
	s.wmu.Lock()
	defer s.wmu.Unlock()
	_ = writeFrame(s.conn, 1, b)
}

func (s *sock) close() {
	s.wmu.Lock()
	_ = writeFrame(s.conn, 8, []byte{0x03, 0xe8})
	s.wmu.Unlock()
	s.conn.Close()
}

func (s *sock) isDead() bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.dead
}

func (s *sock) mark() int { return len(s.snapshot(0)) }

func (s *sock) snapshot(mark int) []Frame {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]Frame(nil), s.frames[mark:]...)
}

func (s *sock) waitAfter(mark int, pred func(Frame) bool, d time.Duration) (Frame, bool) {
	deadline := time.After(d)
	for {
		s.mu.Lock()
		for _, f := range s.frames[mark:] {
			if pred(f) {
				s.mu.Unlock()
				return f, true
			}
		}
		ch := s.ch
		s.mu.Unlock()
		select {
		case <-ch:
		case <-deadline:
			return Frame{}, false
		}
	}
}

// none is true when no frame matching pred arrives within d.
func (s *sock) none(mark int, pred func(Frame) bool, d time.Duration) bool {
	_, got := s.waitAfter(mark, pred, d)
	return !got
}

func (s *sock) count(mark int, pred func(Frame) bool) (n int) {
	for _, f := range s.snapshot(mark) {
		if pred(f) {
			n++
		}
	}
	return
}
