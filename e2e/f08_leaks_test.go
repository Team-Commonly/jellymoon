package e2e

import (
	"encoding/json"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

var (
	secretShape = regexp.MustCompile(`jm[ucr]_[A-Za-z0-9]{10}`)
	// otherShape is every token shape but the computer's: what the computer token's own file may not hold.
	otherShape = regexp.MustCompile(`jm[ur]_[A-Za-z0-9]{10}`)
)

// scan lists where text holds a full secret or something shaped like a token.
func scan(where, text string, shape *regexp.Regexp, secrets []string) (hits []string) {
	for _, s := range secrets {
		if strings.Contains(text, s) {
			hits = append(hits, where+" holds a full token")
		}
	}
	if m := shape.FindString(text); m != "" {
		hits = append(hits, where+" holds a token-shaped string "+m)
	}
	return hits
}

// leaks lists every place a token shows: the captured output of each process and the argv seen while the daemon ran.
func leaks(procs map[string]*dproc, argv string, secrets ...string) []string {
	var hits []string
	for name, p := range procs {
		hits = append(hits, scan(name+" output", p.text(), secretShape, secrets)...)
	}
	return append(hits, scan("argv", argv, secretShape, secrets)...)
}

// fileLeaks walks every regular file under the daemon's config dir, subdirectories and leftover temp files included. The
// one file meant to hold the computer token is tokenFile (computer.json, relative to cfg): there a jmc_ value is expected
// and anything else is a hit. In every other file, any token is a hit. This covers the log too, if the daemon ever writes one.
func fileLeaks(cfg, tokenFile string, secrets ...string) []string {
	var hits []string
	walk := func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			hits = append(hits, path+" could not be read: "+err.Error())
			return nil
		}
		if !d.Type().IsRegular() { // the control socket
			return nil
		}
		rel, _ := filepath.Rel(cfg, path)
		raw, err := os.ReadFile(path)
		if err != nil {
			hits = append(hits, rel+" could not be read: "+err.Error())
			return nil
		}
		shape, held := secretShape, secrets
		if rel == tokenFile {
			shape, held = otherShape, nil
			for _, s := range secrets {
				if !strings.HasPrefix(s, "jmc_") {
					held = append(held, s)
				}
			}
		}
		hits = append(hits, scan(rel, string(raw), shape, held)...)
		return nil
	}
	if err := filepath.WalkDir(cfg, walk); err != nil {
		hits = append(hits, "walking "+cfg+": "+err.Error())
	}
	return hits
}

// envLeaks reads the environment the fake agent recorded for itself: no token may be in it.
func envLeaks(record string, secrets ...string) []string {
	raw, err := os.ReadFile(record)
	if err != nil {
		return []string{"the agent's record " + filepath.Base(record) + " could not be read: " + err.Error()}
	}
	var rec agentRec
	if err := json.Unmarshal(raw, &rec); err != nil {
		return []string{"the agent's record " + filepath.Base(record) + " is not JSON: " + err.Error()}
	}
	return scan("the agent's recorded env", strings.Join(rec.Env, "\n"), secretShape, secrets)
}

// The leak check must see a token written anywhere it should not be. Each case plants one in a place the daemon never
// writes and expects a hit: a mutant of the daemon that copies the token to a second file would be caught the same way.
func TestF08LeakCheckSeesPlantedTokens(t *testing.T) {
	need(t)
	track(t)
	const comp, user = "jmc_PlantedComputerToken0123456789", "jmu_PlantedUserToken0123456789"
	write := func(dir, name, text string) {
		t.Helper()
		if err := os.WriteFile(filepath.Join(dir, name), []byte(text), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	clean := t.TempDir()
	write(clean, "computer.json", `{"token":"`+comp+`"}`)
	write(clean, "agents.json", `[]`)
	if hits := fileLeaks(clean, "computer.json", comp, user); len(hits) != 0 {
		t.Fatalf("the token in its own file and nowhere else is a leak?: %v", hits)
	}
	second := t.TempDir()
	write(second, "computer.json", `{"token":"`+comp+`"}`)
	write(second, "computer.json.copy", comp)
	if hits := fileLeaks(second, "computer.json", comp, user); len(hits) == 0 || !strings.Contains(strings.Join(hits, ";"), "computer.json.copy") {
		t.Errorf("a second file holding the computer token: hits %v, want a hit naming computer.json.copy", hits)
	}
	sub := t.TempDir()
	write(sub, "computer.json", `{"token":"`+comp+`"}`)
	if err := os.Mkdir(filepath.Join(sub, "logs"), 0o700); err != nil {
		t.Fatal(err)
	}
	write(filepath.Join(sub, "logs"), "daemon.log", "started with "+comp[:14]+"0000000000")
	if hits := fileLeaks(sub, "computer.json", comp, user); len(hits) == 0 || !strings.Contains(strings.Join(hits, ";"), "daemon.log") {
		t.Errorf("a token-shaped string in a log under a subdirectory: hits %v, want a hit naming daemon.log", hits)
	}
	wrong := t.TempDir()
	write(wrong, "computer.json", `{"token":"`+comp+`","userToken":"`+user+`"}`)
	if hits := fileLeaks(wrong, "computer.json", comp, user); len(hits) == 0 {
		t.Errorf("the user token inside the computer's own file is a leak, and no hit was reported")
	}
	rec := filepath.Join(t.TempDir(), "agent.json")
	if err := os.WriteFile(rec, []byte(`{"Env":["PATH=/usr/bin","JELLYMOON_X=`+comp+`"]}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if hits := envLeaks(rec, comp, user); len(hits) == 0 {
		t.Errorf("a token in the agent's recorded env: no hit")
	}
}
