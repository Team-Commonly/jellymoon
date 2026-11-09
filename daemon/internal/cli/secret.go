package cli

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// readSecret reads a credential from the environment (and drops it from there), else asks for it on the terminal with echo off.
// `what` names it in the prompt and the errors. It never comes from argv.
func readSecret(envName, what string) (string, error) {
	if v := strings.TrimSpace(os.Getenv(envName)); v != "" {
		_ = os.Unsetenv(envName)
		return v, nil
	}
	tty, err := os.OpenFile("/dev/tty", os.O_RDWR, 0)
	if err != nil {
		return "", fmt.Errorf("no terminal to ask on: set %s to your %s", envName, what)
	}
	defer tty.Close()
	fmt.Fprintf(tty, "%s (not shown): ", what)
	off := exec.Command("stty", "-echo")
	off.Stdin = tty
	if off.Run() != nil {
		return "", fmt.Errorf("cannot hide the input on this terminal: set %s instead", envName)
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
