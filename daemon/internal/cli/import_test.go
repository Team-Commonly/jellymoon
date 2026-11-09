package cli

import (
	"io"
	"strings"
	"testing"
)

// --pace-ms has a floor: a person cannot crawl a fragile instance at full speed.
func TestImportRejectsAPaceBelowTheFloor(t *testing.T) {
	for _, ms := range []string{"0", "100", "249"} {
		err := importCommonly([]string{"--instance", "https://commonly.example", "--pace-ms", ms}, io.Discard, io.Discard)
		if err == nil || !strings.Contains(err.Error(), "--pace-ms") {
			t.Errorf("--pace-ms %s: %v", ms, err)
		}
	}
}
