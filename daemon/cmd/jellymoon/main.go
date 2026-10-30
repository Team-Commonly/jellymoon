// Command jellymoon is the daemon and its command line.
package main

import (
	"os"

	"github.com/Team-Commonly/jellymoon/daemon/internal/cli"
)

func main() { os.Exit(cli.Main(os.Args[1:], os.Stdout, os.Stderr)) }
