package cli

import (
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// poller is the one call the wait loop makes; the server client in use, a fake in tests.
type poller interface {
	DeviceToken(deviceCode string) (protocol.DeviceTokenResponse, error)
}

// login is `jellymoon login` (ADR 0008): the server's device flow. It prints the code and the page to approve it on, waits for
// the approval, and writes the jmu_ token to the config dir's user-token file, which registering a computer reads. The CLI never
// talks to GitHub, and the token goes to that file only: never to argv, stdout, stderr or a log.
func login(stdout io.Writer) error {
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
	d, err := a.StartDevice()
	if err != nil {
		return fmt.Errorf("starting the sign-in: %w", err)
	}
	fmt.Fprintf(stdout, "To sign in, open %s\nand approve the code %s\n", d.VerificationURI, d.UserCode)
	got, err := waitToken(a, d, time.Sleep)
	if err != nil {
		return err
	}
	if err := store.SaveUserToken(got.Token); err != nil {
		return fmt.Errorf("saving the token: %w", err)
	}
	fmt.Fprintf(stdout, "Signed in as %s\n", got.User.DisplayName)
	return nil
}

// slowDown is what a slow_down adds to the wait between polls (RFC 8628, 3.5).
const slowDown = 5 * time.Second

// waitToken polls at the code's interval until the code is approved, and stops on any answer that cannot turn into a token.
func waitToken(p poller, d protocol.DeviceStartResponse, sleep func(time.Duration)) (protocol.DeviceTokenResponse, error) {
	every := time.Duration(max(d.Interval, 1)) * time.Second
	for waited := time.Duration(0); waited <= time.Duration(d.ExpiresIn)*time.Second; waited += every {
		sleep(every)
		out, err := p.DeviceToken(d.DeviceCode)
		if err == nil {
			if !strings.HasPrefix(out.Token, "jmu_") {
				return out, errors.New("the server answered the sign-in without a user token")
			}
			return out, nil
		}
		var e *api.Error
		if !errors.As(err, &e) {
			return out, fmt.Errorf("waiting for the approval: %w", err)
		}
		switch e.Body.Code {
		case protocol.ErrorCodeAuthorizationPending:
		case protocol.ErrorCodeSlowDown:
			every += slowDown
		case protocol.ErrorCodeExpiredToken:
			return out, errors.New("the sign-in code expired before it was approved; run jellymoon login again")
		case protocol.ErrorCodeInvalidGrant:
			return out, errors.New("the sign-in code was already used; run jellymoon login again")
		case protocol.ErrorCodeUnauthorized:
			return out, errors.New("the server refused the sign-in; run jellymoon login again")
		default:
			return out, fmt.Errorf("waiting for the approval: %w", err)
		}
	}
	return protocol.DeviceTokenResponse{}, errors.New("the sign-in code expired before it was approved; run jellymoon login again")
}
