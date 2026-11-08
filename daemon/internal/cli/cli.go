// Package cli is the jellymoon command line: `daemon start --foreground` and `agent add <harness> --pod <id>`.
// Tokens never come from argv: the user token (used once, to register the computer) comes from JELLYMOON_USER_TOKEN or a
// 0600 file in the config dir, the computer token is the daemon's, and the CLI never sees it. Nothing here prints a token.
package cli

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"os/signal"
	"regexp"
	"syscall"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/daemon"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// Environment: the server to talk to, and the jmu_ token for the first registration.
const (
	serverEnv    = "JELLYMOON_SERVER"
	userTokenEnv = "JELLYMOON_USER_TOKEN"
)

const usage = "usage:\n  jellymoon daemon start --foreground\n  jellymoon agent add <harness> --pod <pod id> [--name <display name>]\n"

var podIDShape = regexp.MustCompile(`^pod_[0-9A-Z]{26}$`)

// Main runs one command and returns the exit code.
func Main(args []string, stdout, stderr io.Writer) int {
	var err error
	switch {
	case len(args) >= 2 && args[0] == "daemon" && args[1] == "start":
		err = start(args[2:], stderr)
	case len(args) >= 2 && args[0] == "agent" && args[1] == "add":
		err = agentAdd(args[2:], stdout)
	default:
		fmt.Fprint(stderr, usage)
		return 2
	}
	if err != nil {
		fmt.Fprintf(stderr, "jellymoon: %v\n", err)
		return 1
	}
	return 0
}

// computerFor returns the stored computer, registering one first when the dir has none.
func computerFor(s config.Store, a *api.Client) (config.Computer, error) {
	// The user token is for registering and nothing else. It is dropped on every path, including a restart that never reads it,
	// so nothing the daemon starts later can inherit it.
	defer func() { _ = os.Unsetenv(userTokenEnv) }()
	stored, err := s.Computer()
	if err != nil {
		return config.Computer{}, err
	}
	if stored != nil {
		if stored.Server != a.Base.String() {
			return config.Computer{}, fmt.Errorf("this config dir holds a computer registered on %s, not %s; use another %s", stored.Server, a.Base, config.HomeEnv)
		}
		return *stored, nil
	}
	tok, err := s.UserToken(userTokenEnv)
	if err != nil {
		return config.Computer{}, err
	}
	out, err := a.RegisterComputer(tok)
	if err != nil {
		return config.Computer{}, fmt.Errorf("registering this computer: %w", err)
	}
	c := config.Computer{Server: a.Base.String(), ComputerID: out.ComputerID, Token: out.Token}
	return c, s.SaveComputer(c)
}

func start(args []string, stderr io.Writer) error {
	fs := flag.NewFlagSet("daemon start", flag.ContinueOnError)
	fs.SetOutput(stderr)
	foreground := fs.Bool("foreground", false, "run in this terminal")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if !*foreground {
		return errors.New("only --foreground is supported yet")
	}
	// The server is checked before the config dir is touched and before any token is read.
	server := os.Getenv(serverEnv)
	if server == "" {
		return fmt.Errorf("set %s to the server's URL", serverEnv)
	}
	a, err := api.New(server)
	if err != nil {
		return err
	}
	store, err := config.Open()
	if err != nil {
		return err
	}
	comp, err := computerFor(store, a)
	if err != nil {
		return err
	}
	l := log.New(stderr, "jellymoon: ", log.LstdFlags)
	l.Printf("computer %s on %s", comp.ComputerID, comp.Server)
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	return daemon.New(a, store, comp, l).Run(ctx)
}

func agentAdd(args []string, stdout io.Writer) error {
	if len(args) == 0 {
		return errors.New("agent add needs a harness, such as fake; run jellymoon with no arguments for usage")
	}
	harness := protocol.Harness(args[0])
	fs := flag.NewFlagSet("agent add", flag.ContinueOnError)
	pod := fs.String("pod", "", "the pod to register the agent in")
	name := fs.String("name", string(harness), "the name shown to people")
	if err := fs.Parse(args[1:]); err != nil {
		return err
	}
	if !harness.Valid() {
		return fmt.Errorf("unknown harness %q", harness)
	}
	if !podIDShape.MatchString(*pod) {
		return errors.New("--pod must be a pod id (pod_ and 26 characters)")
	}
	store, err := config.Open()
	if err != nil {
		return err
	}
	id, err := daemon.Add(store, daemon.AddRequest{PodID: *pod, Harness: harness, DisplayName: *name})
	if err != nil {
		return err
	}
	fmt.Fprintf(stdout, "added agent %s (%s) to %s\n", id, *name, *pod)
	return nil
}
