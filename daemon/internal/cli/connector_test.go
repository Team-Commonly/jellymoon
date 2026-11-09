package cli

import (
	"bytes"
	"os"
	"strings"
	"testing"
)

const testPod = "pod_01ARZ3NDEKTSV4RRFFQ69G5FAV"

// runMain runs a command with no server configured: a command that gets as far as the server says so, so any other refusal
// shows the argument checks run first.
func runMain(t *testing.T, args ...string) (int, string) {
	t.Helper()
	t.Setenv(serverEnv, "")
	t.Setenv("JELLYMOON_HOME", t.TempDir())
	var out, errOut bytes.Buffer
	code := Main(args, &out, &errOut)
	return code, out.String() + errOut.String()
}

// The credential leaves the environment as soon as it is read, so nothing the process starts later inherits it.
func TestReadSecretDropsTheEnvironmentVariable(t *testing.T) {
	t.Setenv(connectorTokenEnv, " a-secret-value ")
	v, err := readSecret(connectorTokenEnv, "Upstream token")
	if err != nil || v != "a-secret-value" {
		t.Fatalf("readSecret = %q, %v", v, err)
	}
	if _, ok := os.LookupEnv(connectorTokenEnv); ok {
		t.Errorf("%s is still set after it was read", connectorTokenEnv)
	}
}

// `--access write` is refused before a credential, the server or a request is looked at.
func TestConnectorGrantRefusesPlainWrite(t *testing.T) {
	for _, access := range []string{"write", "admin", ""} {
		code, out := runMain(t, "connector", "grant", "--pod", testPod, "--connection", "con_x", "--agent", "agt_x", "--access", access)
		if code != 1 || !strings.Contains(out, "write-with-confirm") || strings.Contains(out, serverEnv) {
			t.Errorf("--access %q: exit %d, %q; want a refusal that names write-with-confirm and never reaches the server", access, code, out)
		}
	}
}

// A pasted credential, as a stray argument or as the vendor, is refused and never printed.
func TestConnectorConnectNeverEchoesAnArgument(t *testing.T) {
	secret := "pasted" + "-credential-" + "value"
	for _, args := range [][]string{
		{"connector", "connect", "linear", "--pod", testPod, "--url", "https://x.example/mcp", secret},
		{"connector", "connect", secret, "--pod", testPod, "--url", "https://x.example/mcp"},
		{"connector", "connect", "linear", "--pod", testPod, "--url", "https://x.example/mcp", "--token", secret},
	} {
		code, out := runMain(t, args...)
		if code == 0 || strings.Contains(out, secret) {
			t.Errorf("%v: exit %d, output %q; want a refusal that does not echo the argument", args[2:3], code, out)
		}
	}
}

// A credential pasted where the flag parser looks, as a flag name, a flag value it rejects or a malformed flag, is refused in
// general terms with the usage, and the parser never prints it.
func TestConnectorFlagErrorsNeverEchoTheArgument(t *testing.T) {
	secret := "lin" + "_api_" + "EXAMPLE" + "0123456789abcdef"
	base := map[string][]string{
		"connect":   {"connector", "connect", "linear", "--pod", testPod, "--url", "https://x.example/mcp"},
		"grant":     {"connector", "grant", "--pod", testPod, "--connection", "con_x", "--agent", "agt_x", "--access", "read"},
		"approvers": {"connector", "approvers", "--pod", testPod, "--connection", "con_x", "--users", ""},
		"list":      {"connector", "list", "--pod", testPod},
		"receipts":  {"connector", "receipts", "--pod", testPod},
	}
	for name, args := range base {
		for _, stray := range []string{"--" + secret, "-" + secret + "=x", "---" + secret, secret, "--limit=" + secret} {
			code, out := runMain(t, append(append([]string{}, args...), stray)...)
			if code == 0 || strings.Contains(out, "EXAMPLE") {
				t.Errorf("%s with a stray argument: exit %d, output %q; want a refusal that does not echo it", name, code, out)
				continue
			}
			if strings.HasPrefix(stray, "-") && (!strings.Contains(out, "unknown flag") || !strings.Contains(out, "jellymoon connector "+name+" ")) {
				t.Errorf("%s with a stray flag: output %q; want it to say unknown flag and show the usage", name, out)
			}
		}
	}
}

// A --decision that is not a decision id is refused before any request, without being repeated; a well-formed one that
// meets a dead server is reported without the query string.
func TestConnectorReceiptsDecisionNeverEchoed(t *testing.T) {
	// Built from pieces, and not named like a key, so the history scan doesn't read the well-formed id as one.
	pasted := "lin" + "_api_" + "EXAMPLE" + "0123456789abcdef"
	wellFormed := "dec_" + "01JCZK7Q8XG4M2B9" + "V5T3W6RH01"
	for _, decision := range []string{pasted, "dec_" + pasted, wellFormed} {
		t.Setenv("JELLYMOON_HOME", t.TempDir())
		t.Setenv(serverEnv, "http://127.0.0.1:1")
		t.Setenv(userTokenEnv, "jmu_test")
		var out, errOut bytes.Buffer
		code := Main([]string{"connector", "receipts", "--pod", testPod, "--decision", decision, "--limit", "5"}, &out, &errOut)
		all := out.String() + errOut.String()
		if code == 0 || strings.Contains(all, "EXAMPLE") || strings.Contains(all, "decisionId") || strings.Contains(all, decision) {
			t.Errorf("--decision %.8s...: exit %d, %q; want a refusal that names neither the value nor the query", decision, code, all)
		}
		if decision != wellFormed && !strings.Contains(all, "--decision must be a decision id") {
			t.Errorf("--decision %.8s...: %q; want the fixed refusal", decision, all)
		}
	}
}

// Leaving --users out is an error, so a typo cannot clear a connection's approvers; --users "" is how to clear them.
func TestConnectorApproversNeedsUsersFlag(t *testing.T) {
	code, out := runMain(t, "connector", "approvers", "--pod", testPod, "--connection", "con_x")
	if code != 1 || !strings.Contains(out, "--users") {
		t.Errorf("exit %d, %q; want --users required", code, out)
	}
	code, out = runMain(t, "connector", "approvers", "--pod", testPod, "--connection", "con_x", "--users", "")
	if code != 1 || !strings.Contains(out, serverEnv) {
		t.Errorf("--users \"\": exit %d, %q; want it to pass the checks and stop at the missing server", code, out)
	}
}
