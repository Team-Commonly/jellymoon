package cli

import (
	"bytes"
	"os"
	"strings"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
)

// A restart with the computer already stored never reads the user token, and still drops it from the environment.
func TestComputerForDropsTheUserTokenWhenItIsNotRead(t *testing.T) {
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(userTokenEnv, "jmu_secret")
	store, err := config.Open()
	if err != nil {
		t.Fatal(err)
	}
	a, err := api.New("https://example.invalid")
	if err != nil {
		t.Fatal(err)
	}
	if err := store.SaveComputer(config.Computer{Server: a.Base.String(), ComputerID: "cmp_x", Token: "jmc_x"}); err != nil {
		t.Fatal(err)
	}
	if _, err := computerFor(store, a); err != nil {
		t.Fatal(err)
	}
	if v, ok := os.LookupEnv(userTokenEnv); ok {
		t.Errorf("%s is still set (%d chars) after computerFor on a stored computer", userTokenEnv, len(v))
	}
}

// Everything Main writes passes through redaction: a flag error echoes the argument it could not parse, so a token-shaped
// argument and an escape sequence reach neither stream as typed.
func TestMainRedactsWhatItWrites(t *testing.T) {
	var out, errOut bytes.Buffer
	code := Main([]string{"daemon", "start", "--jmu_ABCDEFGHIJ\x1b[2J\r"}, &out, &errOut)
	got := errOut.String()
	if code == 0 {
		t.Fatalf("exit code %d, want a failure for an unknown flag", code)
	}
	if strings.Contains(got, "ABCDEFGHIJ") || strings.ContainsAny(got, "\x1b\r") {
		t.Errorf("stderr carries a token tail or a control character: %q", got)
	}
	if !strings.Contains(got, "jmu_…") {
		t.Errorf("stderr does not show the shortened token: %q", got)
	}
}

// The same holds when the stored computer belongs to another server and the start is refused.
func TestComputerForDropsTheUserTokenWhenItRefuses(t *testing.T) {
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(userTokenEnv, "jmu_secret")
	store, _ := config.Open()
	a, _ := api.New("https://example.invalid")
	_ = store.SaveComputer(config.Computer{Server: "https://other.invalid", ComputerID: "cmp_x", Token: "jmc_x"})
	if _, err := computerFor(store, a); err == nil {
		t.Fatal("want a refusal for a computer registered elsewhere")
	}
	if _, ok := os.LookupEnv(userTokenEnv); ok {
		t.Errorf("%s is still set after a refused start", userTokenEnv)
	}
}

// Asking for help is not an error: every command's --help and -h, and the bare help forms, print and exit 0. The usage names
// every command, login included, and a wrong command still exits 2.
func TestHelpExitsZeroAndTheUsageNamesEveryCommand(t *testing.T) {
	for _, args := range [][]string{
		{"--help"}, {"-h"}, {"help"},
		{"login", "--help"}, {"login", "-h"},
		{"daemon", "start", "--help"}, {"agent", "add", "-h"}, {"import", "commonly", "--help"},
	} {
		var out, errOut bytes.Buffer
		if code := Main(args, &out, &errOut); code != 0 {
			t.Errorf("%v: exit code %d, want 0; stderr %q", args, code, errOut.String())
		}
		if strings.Contains(errOut.String(), "help requested") {
			t.Errorf("%v: printed flag's error %q", args, errOut.String())
		}
	}
	var out, errOut bytes.Buffer
	if code := Main([]string{"--help"}, &out, &errOut); code != 0 || !strings.Contains(out.String(), "jellymoon login\n") {
		t.Errorf("the usage on stdout does not name login: %q", out.String())
	}
	if code := Main([]string{"nope"}, &out, &errOut); code != 2 {
		t.Errorf("an unknown command exits %d, want 2", code)
	}
}
