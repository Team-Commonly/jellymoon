package importer

import (
	"strings"
	"testing"
)

// Token shapes are assembled at runtime so no secret-shaped literal sits in source or history (gitleaks scans both).
// Every shape of credential the importer might meet in a copied field fails the run.
func TestScrubberFailsOnTokenShapes(t *testing.T) {
	bad := map[string]string{
		"commonly device token": "see cm_0123456789abcdef0123 for access",
		"jellymoon user token":  "jmu_ABCDEFGHIJKLMNOP",
		"computer token":        "x jmc_ABCDEFGHIJKLMNOP",
		"run token":             "jmr_ABCDEFGHIJKLMNOP",
		"bearer":                "Authorization: Bearer abcdefgh12345678",
		"sk key":                "sk-" + "abcdefghijklmnopqrstuvwx",
		"github token":          "gh" + "p_" + "abcdefghijklmnopqrstuvwxyz01",
		"aws key":               "AK" + "IA" + "ABCDEFGHIJKLMNOP",
		"jwt":                   "ey" + "JhbGciOiJIUzI1NiJ9." + "ey" + "JzdWIiOiIxMjM0NTY3ODkw." + "abcdefghij",
		"private key":           "-----BEGIN RSA PRIVATE KEY-----",
		"slack webhook":         "https://hooks.slack.com/services/T000/B000/XXXX",
		"discord webhook":       "https://discord.com/api/webhooks/123/abc",
	}
	sc := NewScrubber("")
	for name, v := range bad {
		err := sc.Check("body", v)
		if err == nil {
			t.Errorf("%s: not refused", name)
			continue
		}
		if strings.Contains(err.Error(), v) || strings.Contains(err.Error(), "0123456789abcdef") {
			t.Errorf("%s: the error repeats the secret: %v", name, err)
		}
	}
}

func TestScrubberPassesOrdinaryText(t *testing.T) {
	sc := NewScrubber("")
	for _, v := range []string{"hello team", "we use cm for centimetres", "the cm_ prefix", "a bearer of news", "task-123 done", ""} {
		if err := sc.Check("body", v); err != nil {
			t.Errorf("%q refused: %v", v, err)
		}
	}
}

// The run's own Commonly token is caught by value, whatever shape it has.
func TestScrubberCatchesTheRunsOwnTokenByValue(t *testing.T) {
	sc := NewScrubber("opaque-sentinel-value-1")
	err := sc.Check("summary", "pasted opaque-sentinel-value-1 here")
	if err == nil || strings.Contains(err.Error(), "opaque-sentinel") {
		t.Fatalf("want a refusal that does not repeat the token, got %v", err)
	}
}
