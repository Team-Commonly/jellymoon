package cli

import (
	"bufio"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/importer"
)

// commonlyTokenEnv carries the person's own Commonly token. It never comes from argv, and the importer never reads ~/.commonly.
const commonlyTokenEnv = "COMMONLY_TOKEN"

// dmFlag collects every --include-dm.
type dmFlag []string

func (d *dmFlag) String() string     { return strings.Join(*d, ",") }
func (d *dmFlag) Set(v string) error { *d = append(*d, v); return nil }

// importCommonly is `jellymoon import commonly` (docs/development/import-commonly.md): the person's own Commonly pods and
// messages into their Jellymoon, a dry run unless --apply is given.
func importCommonly(args []string, stdout, stderr io.Writer) error {
	fs := flag.NewFlagSet("import commonly", flag.ContinueOnError)
	fs.SetOutput(stderr)
	instance := fs.String("instance", "", "the Commonly instance, such as https://commonly.example (https; http only on localhost)")
	apply := fs.Bool("apply", false, "create the pods and post the messages (default: a dry run that writes nothing)")
	anon := fs.Bool("anonymize", false, "replace other people's names with participant A, B, ...")
	statePath := fs.String("state", "", "the state file (default: import-commonly.json in the config dir)")
	out := fs.String("out", "jellymoon-import", "where --apply writes the agent cards and the grants checklist")
	paceMS := fs.Int("pace-ms", 260, "the least time between two requests to Commonly, in milliseconds")
	var dms dmFlag
	fs.Var(&dms, "include-dm", "a 1:1 DM pod id to import too (repeatable); DMs are skipped otherwise")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if *instance == "" {
		return errors.New("--instance is required, such as --instance https://commonly.example")
	}
	token, err := commonlyToken()
	if err != nil {
		return err
	}
	c, err := importer.NewCommonly(*instance, token)
	if err != nil {
		return err
	}
	c.SetPace(time.Duration(*paceMS) * time.Millisecond)
	store, err := config.Open()
	if err != nil {
		return err
	}
	o := importer.Options{Apply: *apply, IncludeDM: dms, Anonymize: *anon, StatePath: *statePath, OutDir: *out}
	if o.StatePath == "" {
		o.StatePath = store.Path("import-commonly.json")
	}
	sink, err := importSink(store, *apply, &o)
	if err != nil {
		return err
	}
	rep, err := importer.Run(c, sink, importer.NewScrubber(token), o)
	if err != nil {
		return err
	}
	fmt.Fprint(stdout, rep)
	return nil
}

// importSink is Jellymoon as the person, only when --apply needs it: a dry run touches no Jellymoon at all.
func importSink(store config.Store, apply bool, o *importer.Options) (importer.Sink, error) {
	if !apply {
		return nil, nil
	}
	server := os.Getenv(serverEnv)
	if server == "" {
		return nil, fmt.Errorf("set %s to your Jellymoon server's URL", serverEnv)
	}
	a, err := api.New(server)
	if err != nil {
		return nil, err
	}
	tok, err := store.UserToken(userTokenEnv)
	if err != nil {
		return nil, fmt.Errorf("%w (run jellymoon login first)", err)
	}
	o.Server = a.Base.String()
	return importer.NewSink(a, tok), nil
}

// commonlyToken reads the token from the environment (and drops it from there), else asks for it on the terminal with echo off.
func commonlyToken() (string, error) {
	if v := strings.TrimSpace(os.Getenv(commonlyTokenEnv)); v != "" {
		_ = os.Unsetenv(commonlyTokenEnv)
		return v, nil
	}
	tty, err := os.OpenFile("/dev/tty", os.O_RDWR, 0)
	if err != nil {
		return "", fmt.Errorf("no terminal to ask on: set %s to your Commonly token", commonlyTokenEnv)
	}
	defer tty.Close()
	fmt.Fprint(tty, "Commonly token (not shown): ")
	off := exec.Command("stty", "-echo")
	off.Stdin = tty
	if off.Run() != nil {
		return "", fmt.Errorf("cannot hide the input on this terminal: set %s instead", commonlyTokenEnv)
	}
	line, err := bufio.NewReader(tty).ReadString('\n')
	on := exec.Command("stty", "echo")
	on.Stdin = tty
	_ = on.Run()
	fmt.Fprintln(tty)
	if v := strings.TrimSpace(line); err == nil && v != "" {
		return v, nil
	}
	return "", errors.New("no token was entered")
}
