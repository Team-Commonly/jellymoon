// Package config is the daemon's config dir: the stored computer (with its jmc_ token) and the agents registered through it.
// The dir is 0700 and every file in it 0600, written whole to a temp file in the dir and renamed, so a reader never sees a
// half-written file and the token is never readable by anyone else at any moment.
package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// HomeEnv names the config dir. Without it the dir is jellymoon under the user's config dir.
const HomeEnv = "JELLYMOON_HOME"

// Computer is what registering a computer returned, and the server it belongs to.
type Computer struct {
	Server     string                 `json:"server"`
	ComputerID protocol.ComputerID    `json:"computerId"`
	Token      protocol.ComputerToken `json:"token"`
}

// Agent is an agent registered through this daemon, in one pod.
type Agent struct {
	PodID       protocol.PodID   `json:"podId"`
	AgentID     protocol.AgentID `json:"agentId"`
	Harness     protocol.Harness `json:"harness"`
	DisplayName string           `json:"displayName"`
}

// Store is one config dir.
type Store struct{ Dir string }

// Open resolves the config dir and makes it 0700, creating it (and its parents) when missing.
func Open() (Store, error) {
	dir := os.Getenv(HomeEnv)
	if dir == "" {
		base, err := os.UserConfigDir()
		if err != nil {
			return Store{}, fmt.Errorf("no config dir: set %s", HomeEnv)
		}
		dir = filepath.Join(base, "jellymoon")
	}
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return Store{}, err
	}
	return Store{Dir: dir}, os.Chmod(dir, 0o700)
}

// Path is where a file of the dir lives.
func (s Store) Path(name string) string { return filepath.Join(s.Dir, name) }

func (s Store) write(name string, v any) error {
	raw, err := json.Marshal(v)
	if err != nil {
		return err
	}
	tmp, err := os.CreateTemp(s.Dir, name+".tmp") // created 0600
	if err != nil {
		return err
	}
	_, err = tmp.Write(raw)
	if cerr := tmp.Close(); err == nil {
		err = cerr
	}
	if err == nil {
		err = os.Rename(tmp.Name(), s.Path(name))
	}
	if err != nil {
		_ = os.Remove(tmp.Name())
	}
	return err
}

// read decodes a file into v; a missing file is (false, nil).
func (s Store) read(name string, v any) (bool, error) {
	raw, err := os.ReadFile(s.Path(name))
	if errors.Is(err, os.ErrNotExist) {
		return false, nil
	}
	if err == nil {
		err = json.Unmarshal(raw, v)
	}
	return err == nil, err
}

// Computer returns the stored computer, or nil when this dir has none yet.
func (s Store) Computer() (*Computer, error) {
	var c Computer
	ok, err := s.read("computer.json", &c)
	if !ok || err != nil {
		return nil, err
	}
	return &c, nil
}

// SaveComputer stores the computer. The file is the only copy of its token.
func (s Store) SaveComputer(c Computer) error { return s.write("computer.json", c) }

// Agents returns the agents registered through this daemon.
func (s Store) Agents() ([]Agent, error) {
	var list []Agent
	_, err := s.read("agents.json", &list)
	return list, err
}

// AddAgent records an agent, replacing an earlier record of the same id.
func (s Store) AddAgent(a Agent) error {
	list, err := s.Agents()
	if err != nil {
		return err
	}
	for i := range list {
		if list[i].AgentID == a.AgentID {
			list[i] = a
			return s.write("agents.json", list)
		}
	}
	return s.write("agents.json", append(list, a))
}

// UserToken reads the jmu_ token used once, to register the computer: the env var, else a 0600 file in the dir. Never argv.
// The env var is unset after the read so nothing the daemon starts later inherits it.
func (s Store) UserToken(envName string) (string, error) {
	if v := os.Getenv(envName); v != "" {
		_ = os.Unsetenv(envName)
		return v, nil
	}
	fi, err := os.Stat(s.Path("user-token"))
	if errors.Is(err, os.ErrNotExist) {
		return "", fmt.Errorf("no user token: set %s, or put it in %s (mode 0600)", envName, s.Path("user-token"))
	}
	if err != nil {
		return "", err
	}
	if fi.Mode().Perm()&0o077 != 0 {
		return "", fmt.Errorf("%s is readable by others (mode %v); chmod it to 0600", s.Path("user-token"), fi.Mode().Perm())
	}
	raw, err := os.ReadFile(s.Path("user-token"))
	return string(trimNL(raw)), err
}

func trimNL(b []byte) []byte {
	for len(b) > 0 && (b[len(b)-1] == '\n' || b[len(b)-1] == '\r') {
		b = b[:len(b)-1]
	}
	return b
}
