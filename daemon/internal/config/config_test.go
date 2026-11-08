package config

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

func open(t *testing.T) Store {
	t.Setenv(HomeEnv, filepath.Join(t.TempDir(), "a", "jm"))
	s, err := Open()
	if err != nil {
		t.Fatal(err)
	}
	return s
}

func mode(t *testing.T, p string) os.FileMode {
	fi, err := os.Stat(p)
	if err != nil {
		t.Fatal(err)
	}
	return fi.Mode().Perm()
}

func TestFilesAreOwnerOnly(t *testing.T) {
	s := open(t)
	if err := s.SaveComputer(Computer{Server: "https://x.test", ComputerID: "cmp_1", Token: "jmc_secret"}); err != nil {
		t.Fatal(err)
	}
	if err := s.AddAgent(Agent{PodID: "pod_1", AgentID: "agt_1", Harness: protocol.HarnessFake, DisplayName: "a"}); err != nil {
		t.Fatal(err)
	}
	if err := s.AddAgent(Agent{PodID: "pod_1", AgentID: "agt_1", Harness: protocol.HarnessFake, DisplayName: "renamed"}); err != nil {
		t.Fatal(err)
	}
	for p, want := range map[string]os.FileMode{s.Dir: 0o700, s.Path("computer.json"): 0o600, s.Path("agents.json"): 0o600} {
		if got := mode(t, p); got != want {
			t.Errorf("%s is %v, want %v", filepath.Base(p), got, want)
		}
	}
	if list, _ := s.Agents(); len(list) != 1 || list[0].DisplayName != "renamed" {
		t.Errorf("agents %+v, want one agent, replaced", list)
	}
	if left, _ := filepath.Glob(s.Path("*.tmp*")); len(left) != 0 {
		t.Errorf("temp files left behind: %v", left)
	}
}

func TestOpenTightensAnExistingDir(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "jm")
	if err := os.Mkdir(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	t.Setenv(HomeEnv, dir)
	if _, err := Open(); err != nil || mode(t, dir) != 0o700 {
		t.Errorf("Open err=%v mode=%v, want 0700", err, mode(t, dir))
	}
}

func TestUserToken(t *testing.T) {
	s := open(t)
	t.Setenv("T_UTOK", "jmu_env")
	if v, err := s.UserToken("T_UTOK"); err != nil || v != "jmu_env" || os.Getenv("T_UTOK") != "" {
		t.Errorf("env token %q err=%v left in env=%q; want it read once and unset", v, err, os.Getenv("T_UTOK"))
	}
	if _, err := s.UserToken("T_UTOK"); err == nil {
		t.Error("no token anywhere must be an error")
	}
	if err := os.WriteFile(s.Path("user-token"), []byte("jmu_file\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := s.UserToken("T_UTOK"); err == nil {
		t.Error("a token file others can read must be refused")
	}
	_ = os.Chmod(s.Path("user-token"), 0o600)
	if v, err := s.UserToken("T_UTOK"); err != nil || v != "jmu_file" {
		t.Errorf("file token %q err=%v", v, err)
	}
}
