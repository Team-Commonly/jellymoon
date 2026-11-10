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
		"google": "AI" + "za" + rep("Sy9_b", 7), "gitlab": "gl" + "pat-" + rep("aB3x", 5), "hugging face": "h" + "f_" + rep("aB3xY", 7),
		"stripe": "sk" + "_live_" + rep("aB3x", 6), "stripe restricted": "rk" + "_live_" + rep("aB3x", 6),
	} {
		if got := Credentials("use " + secret + " now"); got != "use "+Replaced+" now" {
			t.Errorf("%s: got %q", name, got)
		}
	}
	if got, want := Credentials("see postgres://app:"+"hunter"+"2secret"+"@"+"db.internal:5432/main and /api/"+rep("aB3x", 12)+"/logs"), "see postgres://app:"+Replaced+"@db.internal:5432/main and /api/"+Replaced+"/logs"; got != want {
		t.Errorf("a password in a URL, and a long mixed segment of a path: got %q", got)
	}
	for _, path := range []string{"/srv/checkouts/workspace/MyProject2/internal/handlers/v2", "src/components/UserProfile2/SettingsPanel3/index", "https://example.com/docs/guide", "mailto:someone@example.com at 10:30"} {
		if got := Credentials("open " + path + " now"); got != "open "+path+" now" {
			t.Errorf("an ordinary path changed: %q", got)
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
