package acp

import "testing"

// A Spec with no Env gives the agent no environment, never the daemon's.
func TestCommandNeverInheritsTheDaemonsEnvironment(t *testing.T) {
	for name, env := range map[string][]string{"nil": nil, "empty": {}} {
		if got := command(Spec{Command: []string{"agent"}, Env: env}).Env; got == nil || len(got) != 0 {
			t.Errorf("%s Env: cmd.Env = %#v, want an empty, non-nil list", name, got)
		}
	}
	if got := command(Spec{Command: []string{"agent"}, Env: []string{"PATH=/bin"}}).Env; len(got) != 1 || got[0] != "PATH=/bin" {
		t.Errorf("cmd.Env = %#v, want the Spec's own", got)
	}
}
