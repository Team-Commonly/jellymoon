package cli

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/reader"
	"github.com/Team-Commonly/jellymoon/daemon/internal/reading"
)

// claudeConfigEnv is Claude Code's own setting for where it keeps its files; unset, that is .claude under the home directory.
const claudeConfigEnv = "CLAUDE_CONFIG_DIR"

// readingTimeout is how long the reading call may take. One measured day took 9 seconds.
const readingTimeout = 3 * time.Minute

// day prints one local day of what the person typed to Claude Code, as a ReaderDay (ADR 0011, section 1). It talks to no
// server and reads no token. The day is a calendar day in the process's time zone, which TZ sets.
func day(args []string, stdout, stderr io.Writer) error {
	out, err := readDay(flag.NewFlagSet("day", flag.ContinueOnError), args, stderr)
	if err != nil {
		return err
	}
	return reading.Encode(stdout, out)
}

// page prints the day's page data (ADR 0011, section 2): the reader's day, read by one reading call through the person's own
// Claude Code and checked by code. It talks to no server and reads no token; the call runs in an empty folder under the config dir.
func page(args []string, stdout, stderr io.Writer) error {
	fs := flag.NewFlagSet("page", flag.ContinueOnError)
	model := fs.String("model", "sonnet", "the model the reading call asks the harness for")
	d, err := readDay(fs, args, stderr)
	if err != nil {
		return err
	}
	store, err := config.Open()
	if err != nil {
		return err
	}
	out, err := reading.Page(d, reading.Options{Model: *model, Dir: store.Path("reading"), Timeout: readingTimeout})
	if err != nil {
		return err
	}
	return reading.Encode(stdout, out)
}

// readDay parses the flags day and page share, on top of any the caller already set on fs, and reads the day.
func readDay(fs *flag.FlagSet, args []string, stderr io.Writer) (protocol.ReaderDay, error) {
	fs.SetOutput(stderr)
	date := fs.String("date", time.Now().Format(time.DateOnly), "the local day to read, as YYYY-MM-DD")
	logs := fs.String("logs", "", "the Claude Code projects directory (default: the harness's own, under your home directory)")
	skip := fs.String("skip", "", "folders to leave out: names, comma separated")
	if err := fs.Parse(args); err != nil {
		return protocol.ReaderDay{}, err
	}
	if fs.NArg() > 0 {
		return protocol.ReaderDay{}, fmt.Errorf("%s takes only flags, not %q", fs.Name(), fs.Arg(0))
	}
	d, err := time.ParseInLocation(time.DateOnly, *date, time.Local)
	if err != nil {
		return protocol.ReaderDay{}, errors.New("--date must be a day as YYYY-MM-DD")
	}
	if *logs == "" {
		config := os.Getenv(claudeConfigEnv)
		if config == "" {
			home, err := os.UserHomeDir()
			if err != nil {
				return protocol.ReaderDay{}, fmt.Errorf("finding the session logs: %w; name them with --logs", err)
			}
			config = filepath.Join(home, ".claude")
		}
		*logs = filepath.Join(config, "projects")
	}
	out, err := reader.Day(reader.Options{Logs: *logs, Day: d, Skip: strings.Split(*skip, ",")})
	if err != nil {
		return out, fmt.Errorf("reading the session logs: %w", err)
	}
	return out, nil
}
