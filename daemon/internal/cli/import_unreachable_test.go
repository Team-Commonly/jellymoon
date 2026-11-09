package cli

import (
	"errors"
	"io"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/importer"
)

func unreachableRun(t *testing.T, base string, extra ...string) error {
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(commonlyTokenEnv, "cm_sentinelTOKEN0001")
	return importCommonly(append([]string{"--instance", base}, extra...), io.Discard, io.Discard)
}

// The command says it can't reach the host, names only the host, and says nothing about saved state on a dry run.
func TestImportOnAClosedPortNamesTheHost(t *testing.T) {
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	addr := l.Addr().String()
	l.Close()
	err = unreachableRun(t, "http://"+addr)
	if err == nil || !strings.Contains(err.Error(), "can't reach "+addr+": connection refused") {
		t.Fatalf("got %v", err)
	}
	if strings.Contains(err.Error(), "saved") || strings.Contains(err.Error(), "sentinel") {
		t.Fatalf("message: %v", err)
	}
}

// "the state is saved" is said only when a state file exists.
func TestStoppedRunClaimsSavedStateOnlyWhenAFileExists(t *testing.T) {
	state := filepath.Join(t.TempDir(), "state.json")
	if err := explainStop(importer.ErrStopped, state); strings.Contains(err.Error(), "saved") || !errors.Is(err, importer.ErrStopped) {
		t.Fatalf("no file: %v", err)
	}
	if err := os.WriteFile(state, []byte(`{}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if err := explainStop(importer.ErrStopped, state); !strings.Contains(err.Error(), "the state is saved") || !errors.Is(err, importer.ErrStopped) {
		t.Fatalf("with a file: %v", err)
	}
	other := errors.New("other")
	if explainStop(other, state) != other {
		t.Fatal("another error was changed")
	}
}
