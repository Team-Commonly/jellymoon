package cli

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/reader"
)

// claudeConfigEnv is Claude Code's own setting for where it keeps its files; unset, that is .claude under the home directory.
const claudeConfigEnv = "CLAUDE_CONFIG_DIR"

// day prints one local day of what the person typed to Claude Code, as a ReaderDay (ADR 0011, section 1). It talks to no
// server and reads no token. The day is a calendar day in the process's time zone, which TZ sets.
func day(args []string, stdout, stderr io.Writer) error {
	fs := flag.NewFlagSet("day", flag.ContinueOnError)
	fs.SetOutput(stderr)
	date := fs.String("date", time.Now().Format(time.DateOnly), "the local day to read, as YYYY-MM-DD")
	logs := fs.String("logs", "", "the Claude Code projects directory (default: the harness's own, under your home directory)")
	skip := fs.String("skip", "", "folders to leave out: names, comma separated")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if fs.NArg() > 0 {
		return fmt.Errorf("day takes only flags, not %q", fs.Arg(0))
	}
	d, err := time.ParseInLocation(time.DateOnly, *date, time.Local)
	if err != nil {
		return errors.New("--date must be a day as YYYY-MM-DD")
	}
	if *logs == "" {
		config := os.Getenv(claudeConfigEnv)
		if config == "" {
			home, err := os.UserHomeDir()
			if err != nil {
				return fmt.Errorf("finding the session logs: %w; name them with --logs", err)
			}
			config = filepath.Join(home, ".claude")
		}
		*logs = filepath.Join(config, "projects")
	}
	out, err := reader.Day(reader.Options{Logs: *logs, Day: d, Skip: strings.Split(*skip, ",")})
	if err != nil {
		return fmt.Errorf("reading the session logs: %w", err)
	}
	enc := json.NewEncoder(stdout)
	enc.SetEscapeHTML(false)
	return enc.Encode(out)
}
