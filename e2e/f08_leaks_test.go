package e2e

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// fileLeaks is not walking yet.
func fileLeaks(cfg, tokenFile string, secrets ...string) []string { return nil }

// envLeaks is not reading records yet.
func envLeaks(record string, secrets ...string) []string { return nil }

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
	if hits := fileLeaks(second, "computer.json", comp, user); len(hits) != 1 || !strings.Contains(hits[0], "computer.json.copy") {
		t.Errorf("a second file holding the computer token: hits %v, want one naming computer.json.copy", hits)
	}
	sub := t.TempDir()
	write(sub, "computer.json", `{"token":"`+comp+`"}`)
	if err := os.Mkdir(filepath.Join(sub, "logs"), 0o700); err != nil {
		t.Fatal(err)
	}
	write(filepath.Join(sub, "logs"), "daemon.log", "started with "+comp[:14]+"0000000000")
	if hits := fileLeaks(sub, "computer.json", comp, user); len(hits) != 1 || !strings.Contains(hits[0], "daemon.log") {
		t.Errorf("a token-shaped string in a log under a subdirectory: hits %v, want one naming daemon.log", hits)
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
	if hits := envLeaks(rec, comp, user); len(hits) != 1 {
		t.Errorf("a token in the agent's recorded env: hits %v, want one", hits)
	}
}
