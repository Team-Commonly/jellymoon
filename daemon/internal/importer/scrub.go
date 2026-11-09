// Package importer is `jellymoon import commonly`: it reads a person's own Commonly pods and messages and posts them to Jellymoon
// under that person's own jmu_ token. It decides no pod rule (ADR 0006): every write is the existing create-pod or post route.
package importer

import (
	"errors"
	"fmt"
	"regexp"
	"strings"
)

// ErrSecret is a run failed by the scrubber. The message names the field, never the value.
var ErrSecret = errors.New("a secret-shaped string was found in text the import would copy")

// shapes are the credentials this importer refuses to carry: this project's own prefixes, a bearer header, and the common key
// formats. A match fails the whole run; nothing is redacted and carried on.
var shapes = map[string]*regexp.Regexp{
	"a cm_ token": regexp.MustCompile(`(?i)\b(?:cm|jmu|jmc|jmr)_[A-Za-z0-9_-]{6,}`),
	// Commonly's real shape (user, cm_agent_, cm_daemon_ and cm_int_ tokens are cm_ + hex). No leading \b, so a glued word does not hide it.
	"a Commonly token":    regexp.MustCompile(`(?i)cm_(?:agent_|daemon_|int_)?[0-9a-f]{32,}`),
	"a bearer credential": regexp.MustCompile(`(?i)\bbearer(?:[\s:]|%20)+[A-Za-z0-9._~+/=-]{8,}`),
	"an sk- key":          regexp.MustCompile(`\bsk-[A-Za-z0-9_-]{16,}`),
	"a GitHub token":      regexp.MustCompile(`\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})`),
	"a Slack token":       regexp.MustCompile(`\bxox[abprs]-[A-Za-z0-9-]{10,}`),
	"an AWS access key":   regexp.MustCompile(`\b(?:AKIA|ASIA)[0-9A-Z]{16}\b`),
	"a Google API key":    regexp.MustCompile(`\bAIza[0-9A-Za-z_-]{30,}`),
	"a JWT":               regexp.MustCompile(`\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{5,}`),
	"a private key block": regexp.MustCompile(`-----BEGIN [A-Z ]*PRIVATE KEY-----`),
	"a Discord bot token": regexp.MustCompile(`\b[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27,}`),
	"a webhook url":       regexp.MustCompile(`(?i)(?:hooks\.slack\.com/services/|discord(?:app)?\.com/api/webhooks/)`),
}

// Scrubber checks every string bound for Jellymoon or an output file.
type Scrubber struct{ literal string }

// NewScrubber also catches the run's own Commonly token by value, whatever shape it has.
func NewScrubber(commonlyToken string) *Scrubber { return &Scrubber{literal: commonlyToken} }

// Check fails with ErrSecret if v holds a token shape or the run's own token. The error carries the field and the kind, not the match.
func (s *Scrubber) Check(field, v string) error {
	if s.literal != "" && strings.Contains(v, s.literal) {
		return fmt.Errorf("%w: %s holds the Commonly token this run was given", ErrSecret, field)
	}
	for kind, re := range shapes {
		if re.MatchString(v) {
			return fmt.Errorf("%w: %s holds %s", ErrSecret, field, kind)
		}
	}
	return nil
}
