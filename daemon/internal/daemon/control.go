package daemon

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// The control socket is how `jellymoon agent add` reaches the running daemon: a unix socket in the config dir (0700, so only
// this user connects), one JSON line each way. The daemon holds the computer token, so the CLI never reads or sends it and no
// token is in any argv. The socket path must fit sun_path, the 104 bytes macOS allows; a longer config dir is refused.
const (
	socketName = "d.sock"
	maxSocket  = 100
)

// AddRequest asks the running daemon to register an agent of this computer in a pod.
type AddRequest struct {
	PodID       protocol.PodID   `json:"podId"`
	Harness     protocol.Harness `json:"harness"`
	DisplayName string           `json:"displayName"`
}

type addReply struct {
	AgentID protocol.AgentID `json:"agentId,omitempty"`
	Error   string           `json:"error,omitempty"`
}

func socketPath(s config.Store) (string, error) {
	p := s.Path(socketName)
	if len(p) > maxSocket {
		return "", fmt.Errorf("the config dir path is too long for a unix socket (%d bytes, limit %d); set %s to a shorter one", len(p), maxSocket, config.HomeEnv)
	}
	return p, nil
}

// listen opens the control socket. A socket someone answers on means a daemon already runs here; one nobody answers on is stale.
func (d *Daemon) listen() (net.Listener, error) {
	p, err := socketPath(d.store)
	if err != nil {
		return nil, err
	}
	if c, err := net.DialTimeout("unix", p, time.Second); err == nil {
		c.Close()
		return nil, errors.New("a daemon already runs on this config dir")
	}
	_ = os.Remove(p)
	l, err := net.Listen("unix", p)
	if err == nil {
		err = os.Chmod(p, 0o600)
	}
	return l, err
}

func (d *Daemon) serve(l net.Listener) {
	for {
		c, err := l.Accept()
		if err != nil {
			return
		}
		go d.handle(c)
	}
}

func (d *Daemon) handle(c net.Conn) {
	defer c.Close()
	_ = c.SetDeadline(time.Now().Add(30 * time.Second))
	var req AddRequest
	var rep addReply
	if err := json.NewDecoder(bufio.NewReader(c)).Decode(&req); err != nil {
		rep.Error = "unreadable request"
	} else if id, err := d.add(req); err != nil {
		rep.Error = err.Error()
	} else {
		rep.AgentID = id
	}
	_ = json.NewEncoder(c).Encode(rep)
}

// add registers the agent with the pod, remembers it so a restart reconnects, and opens the pod socket.
func (d *Daemon) add(req AddRequest) (protocol.AgentID, error) {
	id, err := d.api.RegisterAgent(d.comp.Token, req.PodID, req.Harness, req.DisplayName)
	if err != nil {
		return "", err
	}
	if err := d.store.AddAgent(config.Agent{PodID: req.PodID, AgentID: id, Harness: req.Harness, DisplayName: req.DisplayName}); err != nil {
		return "", err
	}
	d.log.Printf("agent %s registered in pod %s", id, req.PodID)
	d.watch(req.PodID)
	return id, nil
}

// Add asks the daemon running on the store to register an agent, and returns its id.
func Add(s config.Store, req AddRequest) (protocol.AgentID, error) {
	p, err := socketPath(s)
	if err != nil {
		return "", err
	}
	c, err := net.DialTimeout("unix", p, 3*time.Second)
	if err != nil {
		return "", errors.New("no daemon is running on this config dir; start one with `jellymoon daemon start --foreground`")
	}
	defer c.Close()
	_ = c.SetDeadline(time.Now().Add(30 * time.Second))
	if err := json.NewEncoder(c).Encode(req); err != nil {
		return "", err
	}
	var rep addReply
	if err := json.NewDecoder(bufio.NewReader(c)).Decode(&rep); err != nil {
		return "", err
	}
	if rep.Error != "" {
		return "", errors.New(rep.Error)
	}
	return rep.AgentID, nil
}
