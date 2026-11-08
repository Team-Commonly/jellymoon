// Package ws is a small RFC 6455 client: text frames, ping and pong, close, and fragmented messages. Standard library only.
// The pod sends whole text frames, so this is the least that carries the pod socket; it is not a general WebSocket library.
package ws

import (
	"bufio"
	"context"
	"crypto/rand"
	"crypto/sha1"
	"crypto/tls"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"sync"
	"time"
)

const (
	guid       = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
	maxMessage = 1 << 20
	opText     = 1
	opClose    = 8
	opPing     = 9
	opPong     = 10
)

// Conn is one open socket. Send may be called from several goroutines; Read from one.
type Conn struct {
	c   net.Conn
	br  *bufio.Reader
	wmu sync.Mutex
}

// Dial upgrades an http(s) URL with a bearer token in Authorization, as the pod socket requires.
func Dial(ctx context.Context, u *url.URL, token string) (*Conn, error) {
	addr := u.Host
	if u.Port() == "" {
		addr = net.JoinHostPort(u.Hostname(), map[string]string{"http": "80", "https": "443"}[u.Scheme])
	}
	d := &net.Dialer{Timeout: 10 * time.Second}
	var c net.Conn
	var err error
	if u.Scheme == "https" {
		c, err = (&tls.Dialer{NetDialer: d, Config: &tls.Config{ServerName: u.Hostname(), MinVersion: tls.VersionTLS12}}).DialContext(ctx, "tcp", addr)
	} else {
		c, err = d.DialContext(ctx, "tcp", addr)
	}
	if err != nil {
		return nil, err
	}
	conn := &Conn{c: c, br: bufio.NewReader(c)}
	if err := conn.handshake(u, token); err != nil {
		c.Close()
		return nil, err
	}
	return conn, nil
}

func (c *Conn) handshake(u *url.URL, token string) error {
	raw := make([]byte, 16)
	_, _ = rand.Read(raw)
	key := base64.StdEncoding.EncodeToString(raw)
	req, err := http.NewRequest(http.MethodGet, u.String(), nil)
	if err != nil {
		return err
	}
	for k, v := range map[string]string{"Upgrade": "websocket", "Connection": "Upgrade", "Sec-WebSocket-Key": key, "Sec-WebSocket-Version": "13", "Authorization": "Bearer " + token} {
		req.Header.Set(k, v)
	}
	_ = c.c.SetDeadline(time.Now().Add(10 * time.Second))
	defer func() { _ = c.c.SetDeadline(time.Time{}) }()
	if err := req.Write(c.c); err != nil {
		return err
	}
	resp, err := http.ReadResponse(c.br, req)
	if err != nil {
		return err
	}
	if resp.StatusCode != http.StatusSwitchingProtocols {
		b, _ := io.ReadAll(io.LimitReader(resp.Body, 300))
		return fmt.Errorf("upgrade refused: HTTP %d %s", resp.StatusCode, b)
	}
	sum := sha1.Sum([]byte(key + guid))
	if resp.Header.Get("Sec-WebSocket-Accept") != base64.StdEncoding.EncodeToString(sum[:]) {
		return errors.New("upgrade answered with a bad Sec-WebSocket-Accept")
	}
	return nil
}

func (c *Conn) frame(op byte, p []byte) error {
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
	c.wmu.Lock()
	defer c.wmu.Unlock()
	_ = c.c.SetWriteDeadline(time.Now().Add(10 * time.Second))
	_, err := c.c.Write(out)
	return err
}

// Send writes one text message.
func (c *Conn) Send(p []byte) error { return c.frame(opText, p) }

// Close says goodbye and closes the connection.
func (c *Conn) Close() error {
	_ = c.frame(opClose, []byte{0x03, 0xe8})
	return c.c.Close()
}

func (c *Conn) readFrame() (fin bool, op byte, p []byte, err error) {
	var h [2]byte
	if _, err = io.ReadFull(c.br, h[:]); err != nil {
		return
	}
	fin, op = h[0]&0x80 != 0, h[0]&0x0f
	n := uint64(h[1] & 0x7f)
	if n >= 126 {
		ext := make([]byte, map[uint64]int{126: 2, 127: 8}[n])
		if _, err = io.ReadFull(c.br, ext); err != nil {
			return
		}
		n = 0
		for _, b := range ext {
			n = n<<8 | uint64(b)
		}
	}
	if n > maxMessage {
		err = fmt.Errorf("a frame of %d bytes is over the %d limit", n, maxMessage)
		return
	}
	p = make([]byte, n)
	_, err = io.ReadFull(c.br, p) // a server never masks
	return
}

// Read returns the next text message. It answers pings itself, and returns an error once the socket is closed or fails.
func (c *Conn) Read() ([]byte, error) {
	var msg []byte
	for {
		fin, op, p, err := c.readFrame()
		if err != nil {
			return nil, err
		}
		switch op {
		case opClose:
			return nil, io.EOF
		case opPing:
			if err := c.frame(opPong, p); err != nil {
				return nil, err
			}
		case opPong:
		default: // text, binary or continuation
			if msg = append(msg, p...); len(msg) > maxMessage {
				return nil, errors.New("a message is over the size limit")
			}
			if fin {
				return msg, nil
			}
		}
	}
}
