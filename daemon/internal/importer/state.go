package importer

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

// State is the local record of what was imported. It holds ids and timestamps only: never a token, a name or a message.
//
// Messages are posted oldest first, so "imported" is a watermark: everything older than Cursor is done, and AtCursor lists the
// ids already posted at exactly the cursor's timestamp. Inflight is the one message being posted when the run last saved: if the
// run died there, the next run looks for its marker in the Jellymoon pod before posting it again.
type State struct {
	Version  int                  `json:"version"`
	Instance string               `json:"instance"`
	Server   string               `json:"server"`
	Pods     map[string]*PodState `json:"pods"`
	Anon     map[string]string    `json:"anon,omitempty"`
}

// PodState is one Commonly pod's progress. JM is the Jellymoon pod it maps to (the source id commonly-pod:<id> is its key).
type PodState struct {
	JM       string   `json:"jm"`
	Cursor   string   `json:"cursor,omitempty"`
	AtCursor []string `json:"atCursor,omitempty"`
	Inflight string   `json:"inflight,omitempty"`
}

// done reports whether a message at `at` with id `id` was already posted.
func (p *PodState) done(at, id string) bool {
	if p == nil || p.Cursor == "" || at > p.Cursor {
		return false
	}
	if at < p.Cursor {
		return true
	}
	for _, d := range p.AtCursor {
		if d == id {
			return true
		}
	}
	return false
}

// advance records a posted message and clears the in-flight mark.
func (p *PodState) advance(at, id string) {
	p.Inflight = ""
	if at != p.Cursor {
		p.Cursor, p.AtCursor = at, nil
	}
	p.AtCursor = append(p.AtCursor, id)
}

// LoadState reads the state file; a missing file is an empty state. A state from another instance or server is refused.
func LoadState(path, instance, server string) (*State, error) {
	st := &State{Version: 1, Instance: instance, Server: server, Pods: map[string]*PodState{}, Anon: map[string]string{}}
	raw, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return st, nil
	}
	if err != nil {
		return nil, err
	}
	var got State
	if err := json.Unmarshal(raw, &got); err != nil || got.Version != 1 {
		return nil, fmt.Errorf("%s is not an import state file this version reads", path)
	}
	if got.Instance != instance || (server != "" && got.Server != "" && got.Server != server) {
		return nil, fmt.Errorf("%s belongs to another Commonly instance or Jellymoon server; use --state with another file", path)
	}
	if got.Pods == nil {
		got.Pods = map[string]*PodState{}
	}
	if got.Anon == nil {
		got.Anon = map[string]string{}
	}
	got.Server = server
	return &got, nil
}

// Save writes the state whole to a temp file in its directory and renames it, 0600, so a kill never leaves half a file.
func (s *State) Save(path string) error {
	raw, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return err
	}
	return writeOwnerOnly(path, raw)
}

func writeOwnerOnly(path string, raw []byte) error {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return err
	}
	tmp, err := os.CreateTemp(dir, ".import-*")
	if err != nil {
		return err
	}
	defer os.Remove(tmp.Name())
	if err := tmp.Chmod(0o600); err != nil {
		tmp.Close()
		return err
	}
	if _, err := tmp.Write(raw); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	return os.Rename(tmp.Name(), path)
}
