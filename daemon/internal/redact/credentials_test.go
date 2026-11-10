package redact

import (
	"strings"
	"testing"
)

// Credentials replaces vendor key shapes and long mixed runs, and leaves a commit hash, a path and plain words alone. The shapes
// are joined here so no scanner reads one in this file.
func TestCredentialsReplacesKeyShapes(t *testing.T) {
	rep := strings.Repeat
	for name, secret := range map[string]string{
		"model key": "sk" + "-proj-" + rep("aB3x", 6), "hub": "gh" + "o_" + rep("aB3x", 6), "hub pat": "github" + "_pat_" + rep("aB3_", 6),
		"slack": "xo" + "xb-" + rep("1234-", 3) + "abcd", "aws id": "AK" + "IA" + rep("Q7", 8), "jwt": "ey" + "J" + rep("hbGci", 3) + "." + rep("eyJzd", 3) + ".sig",
		"pem": "-----BEGIN RSA " + "PRIVATE KEY-----\nMIIEabc\n-----END RSA " + "PRIVATE KEY-----", "unknown make": rep("aB3+/x9Z", 5),
	} {
		if got := Credentials("use " + secret + " now"); got != "use "+Replaced+" now" {
			t.Errorf("%s: got %q", name, got)
		}
	}
	plain := "commit " + rep("0123456789abcdef", 3) + " in docs/development/a-long-folder-name/another-long-folder-name/file.md, sk-short, ghp_x"
	if got := Credentials(plain); got != plain {
		t.Errorf("text with no credential changed: %q", got)
	}
	if got := Credentials("a jmr_AbC123 token\x1b[2J"); got != "a jmr_… token[2J" {
		t.Errorf("Credentials must also do what String does: %q", got)
	}
}

func TestStringTwiceIsStringOnce(t *testing.T) {
	once := String("jmu_AbC123 and a bare jmr_ and cm_0123")
	if twice := String(once); twice != once || once != "jmu_… and a bare jmr_… and cm_…" {
		t.Errorf("once %q, twice %q", once, twice)
	}
}
