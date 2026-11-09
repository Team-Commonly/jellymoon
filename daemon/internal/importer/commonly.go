package importer

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/rand/v2"
	"net"
	"net/http"
	"net/url"
	"regexp"
	"strconv"
	"strings"
	"syscall"
	"time"
	"unicode"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
)

// ErrStopped is five consecutive failures in a row. The run stops and does not retry on its own. The text says nothing about saved
// state: the command adds that only when a state file exists.
var ErrStopped = errors.New("stopped after 5 consecutive failures; run the command again later")

// UnreachableError is a host that refused the connection or does not resolve, before it has answered anything. Only the host and the
// class are kept, never the raw error text, which could carry the URL.
type UnreachableError struct{ Host, Reason string }

func (e *UnreachableError) Error() string { return fmt.Sprintf("can't reach %s: %s", e.Host, e.Reason) }

const (
	maxConsecutiveFailures = 5
	backoffBase            = 500 * time.Millisecond
	backoffCap             = 30 * time.Second
	retryAfterCap          = 2 * time.Minute
	maxBody                = 16 << 20
)

// readable is every route the importer calls, as a pattern. A path that matches none is refused before any request is made, so a
// route that returns another person's data, a credential or a runtime token cannot be reached by a coding slip.
var readable = []*regexp.Regexp{
	regexp.MustCompile(`^/api/auth/user$`),
	regexp.MustCompile(`^/api/pods$`),
	regexp.MustCompile(`^/api/pods/[^/]+$`),
	regexp.MustCompile(`^/api/messages/[^/]+$`),
	regexp.MustCompile(`^/api/grants$`),
	regexp.MustCompile(`^/api/integrations/[^/]+$`),
}

// Commonly is a read-only client for one Commonly instance and one person's token. The token lives in this struct and is sent as
// the Authorization header of a GET; it is in no error, no log line and no file.
type Commonly struct {
	base  *url.URL
	token string
	hc    *http.Client
	fails int
	// sleep, gap and jitter are replaced by tests.
	sleep  func(time.Duration)
	gap    time.Duration
	jitter func() float64
	last   time.Time
	// answered is true once the instance has replied with any HTTP status.
	answered bool
}

// NewCommonly checks the instance URL the way the daemon checks its server: https, or http on localhost or 127.0.0.1. Redirects
// are never followed, so the token cannot travel to another host.
func NewCommonly(rawURL, token string) (*Commonly, error) {
	base, err := api.CheckServer(rawURL)
	if err != nil {
		return nil, err
	}
	// A token with a space or a line break inside it can never be right; say so before any request, without echoing it. The CLI's
	// readSecret has already trimmed whitespace around it (a trailing newline from a file), as it does for every credential.
	if strings.IndexFunc(token, func(r rune) bool { return unicode.IsSpace(r) || unicode.IsControl(r) }) >= 0 {
		return nil, errors.New("COMMONLY_TOKEN has spaces or line breaks in it; paste the token alone")
	}
	hc := &http.Client{Timeout: 30 * time.Second, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	return &Commonly{base: base, token: token, hc: hc, sleep: time.Sleep, gap: 260 * time.Millisecond, jitter: rand.Float64}, nil
}

// MinPace is the floor for --pace-ms: below it a run could crawl a fragile instance at full speed. The default stays under the
// instance's 240 reads a minute on messages.
const MinPace = 250 * time.Millisecond

// SetPace sets the least time between two requests.
func (c *Commonly) SetPace(d time.Duration) { c.gap = d }

// Host names the instance in the state file, so a state from one instance is never applied to another.
func (c *Commonly) Host() string { return c.base.Host }

// explainIdentity turns a refusal of the identity check into what the person can act on. The token is never in the text.
func (c *Commonly) explainIdentity(err error) error {
	var se *StatusError
	if !errors.As(err, &se) {
		return err
	}
	switch se.Status {
	case 401, 403:
		return fmt.Errorf("Commonly refused this token (%d): check COMMONLY_TOKEN", se.Status)
	case 404:
		return fmt.Errorf("%s doesn't look like a Commonly instance (no /api/auth/user)", c.base.String())
	}
	return fmt.Errorf("%s answered the identity check with HTTP %d", c.base.Host, se.Status)
}

// Get is one read. It retries a 429, a 5xx and a transport failure with jittered exponential backoff (Retry-After wins when
// longer), and after five failures in a row returns ErrStopped. Any other status is a *StatusError and is never retried.
func (c *Commonly) Get(path string, q url.Values, out any) error {
	if err := allowed(path, q); err != nil {
		return err
	}
	target := c.base.String() + path
	if len(q) > 0 {
		target += "?" + q.Encode()
	}
	route := routeOf(path)
	for {
		c.pace()
		status, retryAfter, body, err := c.once(target)
		if err == nil {
			c.answered = true
		} else if reason := unreachableReason(err); reason != "" && !c.answered {
			return &UnreachableError{Host: c.base.Host, Reason: reason}
		}
		switch {
		case err == nil && status >= 200 && status < 300:
			c.fails = 0
			if out == nil {
				return nil
			}
			if err := json.Unmarshal(body, out); err != nil {
				return fmt.Errorf("GET %s: the answer is not the JSON the importer expects", route)
			}
			return nil
		case err == nil && status != 429 && status < 500:
			c.fails = 0
			return &StatusError{Path: route, Status: status}
		}
		if c.fails++; c.fails >= maxConsecutiveFailures {
			return ErrStopped
		}
		c.sleep(c.backoff(retryAfter))
	}
}

var idRoute = regexp.MustCompile(`^(/api/(?:pods|messages|integrations))/[^/]+$`)

// routeOf is the route shape of a path, with the pod id left out: an error names "/api/messages/:podId", never an id that could be secret-shaped.
func routeOf(path string) string { return idRoute.ReplaceAllString(path, "$1/:podId") }

// allowed is the client's own gate: a listed route, and never a scope parameter, in any letter case.
func allowed(path string, q url.Values) error {
	for k := range q {
		if strings.EqualFold(k, "scope") {
			return errors.New("internal error: the importer never sends a scope parameter")
		}
	}
	for _, re := range readable {
		if re.MatchString(path) {
			return nil
		}
	}
	return fmt.Errorf("internal error: %s is not a route the importer reads", routeOf(path))
}

func (c *Commonly) pace() {
	if wait := c.gap - time.Since(c.last); wait > 0 {
		c.sleep(wait)
	}
	c.last = time.Now()
}

// once makes one request. A transport error is returned without the URL or any header.
func (c *Commonly) once(target string) (status int, retryAfter time.Duration, body []byte, err error) {
	req, err := http.NewRequest(http.MethodGet, target, nil)
	if err != nil {
		return 0, 0, nil, errors.New("bad request")
	}
	req.Header.Set("Authorization", "Bearer "+c.token)
	req.Header.Set("Accept", "application/json")
	req.Header.Set("User-Agent", "jellymoon-import/1")
	resp, err := c.hc.Do(req)
	if err != nil {
		return 0, 0, nil, &transportError{reason: transportClass(err)}
	}
	defer resp.Body.Close()
	body, err = io.ReadAll(io.LimitReader(resp.Body, maxBody))
	if err != nil {
		return 0, 0, nil, errors.New("the answer broke off")
	}
	return resp.StatusCode, parseRetryAfter(resp.Header.Get("Retry-After")), body, nil
}

// transportError is a failed request reduced to its class; the raw error is dropped because it can name the URL.
type transportError struct{ reason string }

func (e *transportError) Error() string { return "the request failed" }

// transportClass names the class of a transport error. Only a refusal and a missing name are ever called unreachable.
func transportClass(err error) string {
	var dns *net.DNSError
	switch {
	case errors.Is(err, syscall.ECONNREFUSED):
		return "connection refused"
	case errors.As(err, &dns) && !dns.IsTimeout && !dns.IsTemporary:
		return "the name doesn't resolve"
	}
	return ""
}

func unreachableReason(err error) string {
	var te *transportError
	if errors.As(err, &te) {
		return te.reason
	}
	return ""
}

// parseRetryAfter reads both forms, seconds and an HTTP date, and caps the wait.
func parseRetryAfter(v string) time.Duration {
	var d time.Duration
	if secs, err := strconv.Atoi(v); err == nil {
		d = time.Duration(secs) * time.Second
	} else if at, err := http.ParseTime(v); err == nil {
		d = time.Until(at)
	}
	return min(max(d, 0), retryAfterCap)
}

func (c *Commonly) backoff(retryAfter time.Duration) time.Duration {
	d := min(backoffBase<<(c.fails-1), backoffCap)
	d = d/2 + time.Duration(c.jitter()*float64(d/2))
	return max(d, retryAfter)
}

// StatusError (its Path is the route shape, never a raw path) is a refusal the importer does not retry: 401, 403, 404 and the like.
type StatusError struct {
	Path   string
	Status int
}

func (e *StatusError) Error() string { return fmt.Sprintf("GET %s: HTTP %d", e.Path, e.Status) }
