package cli

import (
	"errors"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// script answers each poll with the next code; "" is a token.
type script struct {
	codes []protocol.ErrorCode
	calls int
}

func (s *script) DeviceToken(string) (protocol.DeviceTokenResponse, error) {
	c := s.codes[s.calls]
	s.calls++
	if c == "" {
		return protocol.DeviceTokenResponse{Token: "jmu_x", User: protocol.Actor{DisplayName: "octo"}}, nil
	}
	return protocol.DeviceTokenResponse{}, &api.Error{Path: "/v1/auth/device/token", Status: 400, Body: protocol.ErrorBody{Code: c}}
}

func run(codes ...protocol.ErrorCode) (*script, []time.Duration, error) {
	s := &script{codes: codes}
	var slept []time.Duration
	_, err := waitToken(s, protocol.DeviceStartResponse{DeviceCode: "d", Interval: 2, ExpiresIn: 900}, func(d time.Duration) { slept = append(slept, d) })
	return s, slept, err
}

func TestWaitTokenPollsAtTheIntervalAndBacksOffOnSlowDown(t *testing.T) {
	s, slept, err := run(protocol.ErrorCodeAuthorizationPending, protocol.ErrorCodeSlowDown, protocol.ErrorCodeAuthorizationPending, "")
	want := []time.Duration{2 * time.Second, 2 * time.Second, 7 * time.Second, 7 * time.Second}
	if err != nil || s.calls != 4 || !slices.Equal(slept, want) {
		t.Fatalf("err %v, %d polls, slept %v; want a token after 4 polls, slept %v", err, s.calls, slept, want)
	}
}

func TestWaitTokenStopsOnEachTerminalAnswer(t *testing.T) {
	for code, says := range map[protocol.ErrorCode]string{
		protocol.ErrorCodeExpiredToken: "expired", protocol.ErrorCodeInvalidGrant: "already used",
		protocol.ErrorCodeUnauthorized: "refused", protocol.ErrorCodeInternal: "internal",
	} {
		s, _, err := run(protocol.ErrorCodeAuthorizationPending, code, "")
		if err == nil || s.calls != 2 || !strings.Contains(err.Error(), says) {
			t.Errorf("%s: err %v after %d polls; want it to stop at once, saying %q", code, err, s.calls, says)
		}
	}
}

type down struct{}

func (down) DeviceToken(string) (protocol.DeviceTokenResponse, error) {
	return protocol.DeviceTokenResponse{}, errors.New("connection refused")
}

func TestWaitTokenStopsAtTheExpiryAndOnATransportError(t *testing.T) {
	pending := make([]protocol.ErrorCode, 1000)
	for i := range pending {
		pending[i] = protocol.ErrorCodeAuthorizationPending
	}
	s := &script{codes: pending}
	_, err := waitToken(s, protocol.DeviceStartResponse{Interval: 5, ExpiresIn: 60}, func(time.Duration) {})
	if err == nil || !strings.Contains(err.Error(), "expired") || s.calls != 13 {
		t.Fatalf("never approved: err %v after %d polls; want it to give up at the expiry, after 13", err, s.calls)
	}
	if _, err := waitToken(down{}, protocol.DeviceStartResponse{Interval: 1, ExpiresIn: 60}, func(time.Duration) {}); err == nil || !strings.Contains(err.Error(), "connection refused") {
		t.Fatalf("a transport error: %v", err)
	}
}

func TestWaitTokenStopsPlainlyWhenTheCodeIsRefusedInTheBrowser(t *testing.T) {
	const says = "the sign-in was refused in the browser"
	s, _, err := run(protocol.ErrorCodeAuthorizationPending, protocol.ErrorCodeAccessDenied, "")
	if err == nil || err.Error() != says || s.calls != 2 {
		t.Fatalf("access_denied: err %v after %d polls; want exactly %q and no further poll", err, s.calls, says)
	}
}
