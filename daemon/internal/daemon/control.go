package daemon

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"regexp"
	"strings"
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
	Repo        string           `json:"repo,omitempty"` // absolute path of a git repo the agent's runs get a worktree of
	// AllowPermissions is the agent's allow setting (config.Agent).
	AllowPermissions bool `json:"allowPermissions,omitempty"`
}

type addReply struct {
	AgentID protocol.AgentID `json:"agentId,omitempty"`
	Error   string           `json:"error,omitempty"`
}

// A person addresses an agent with @name, and the pod core reads a mention as `@([\w.-]+)` with trailing dots and dashes dropped
// (podcore/src/board.ts, mentioned), matching it exactly against the display name. So a display name that can be mentioned is
// ASCII letters, digits, _ . and -, and does not end in . or -. Any other name is registered but can never wake its agent.
var (
	mentionable   = regexp.MustCompile(`^[A-Za-z0-9_.-]*[A-Za-z0-9_]$`)
	notMentionRun = regexp.MustCompile(`[^A-Za-z0-9_.-]+`)
)

// CheckDisplayName refuses a display name no @mention can address, naming the allowed characters and suggesting one.
func CheckDisplayName(name string) error {
	if mentionable.MatchString(name) {
		return nil
	}
	const rule = "an agent's name must be letters, digits, _ . and -, ending in a letter, digit or _, so that @name can reach it"
	fix := strings.Trim(notMentionRun.ReplaceAllString(strings.TrimSpace(name), "-"), ".-")
	if fix == "" {
		return fmt.Errorf("%q is not a name people can mention: %s", name, rule)
	}
	return fmt.Errorf("%q is not a name people can mention: %s; try %q", name, rule, fix)
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

func (d *Daemon) serve(l net.Listener) { d.serveAs(l, uint32(os.Geteuid())) }

// serveAs accepts connections; self is the uid they must come from.
func (d *Daemon) serveAs(l net.Listener, self uint32) {
	for {
		c, err := l.Accept()
		if err != nil {
			return
		}
		if !permit(c, self) {
			c.Close() // no reply: a stranger learns nothing, not even that a daemon answers
			continue
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
	if err := CheckDisplayName(req.DisplayName); err != nil {
		return "", err
	}
	// The fake harness is the test agent and takes its program when it wakes; any other harness needs its program set now, in the
	// daemon's environment, or every wake of the agent would fail. Refuse before registering, so no agent row exists.
	if req.Harness != protocol.HarnessFake {
		if _, err := harnessCommand(req.Harness); err != nil {
			return "", err
		}
	}
	if req.Repo != "" {
		if fi, err := os.Stat(req.Repo); !filepath.IsAbs(req.Repo) || err != nil || !fi.IsDir() {
			return "", fmt.Errorf("--repo %q must be the absolute path of a directory", req.Repo)
		}
	}
	id, err := d.api.RegisterAgent(d.comp.Token, req.PodID, req.Harness, req.DisplayName)
	if err != nil {
		return "", err
	}
	// AddAgent reads, changes and rewrites agents.json, so one add at a time; watch takes d.mu again only after this.
	d.mu.Lock()
	err = d.store.AddAgent(config.Agent{PodID: req.PodID, AgentID: id, Harness: req.Harness, DisplayName: req.DisplayName, Repo: req.Repo, AllowPermissions: req.AllowPermissions})
	d.mu.Unlock()
	if err != nil {
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
