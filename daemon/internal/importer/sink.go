package importer

import (
	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// jellymoon is the Sink backed by the Jellymoon server, as the person who ran the command.
type jellymoon struct {
	a   *api.Client
	tok string
}

// NewSink writes through the existing routes with the person's jmu_ token. The token is held for the run and never printed.
func NewSink(a *api.Client, userToken string) Sink { return jellymoon{a: a, tok: userToken} }

func (j jellymoon) CreatePod(name string) (string, error) {
	p, err := j.a.CreatePod(j.tok, name)
	return p.ID, err
}

func (j jellymoon) Post(podID string, in protocol.PostInput) error {
	_, err := j.a.PostMessage(j.tok, podID, in)
	return err
}

// Sources lists the source markers already in a pod, reading its feed 50 at a time. It runs only when a run died between a post and its answer.
func (j jellymoon) Sources(podID string) (map[string]bool, error) {
	out := map[string]bool{}
	for since := int64(0); ; {
		r, err := j.a.Feed(j.tok, podID, since)
		if err != nil {
			return nil, err
		}
		next := since
		for _, m := range r.Messages {
			next = max(next, m.Seq)
			if m.Body != nil {
				out[markerOf(*m.Body)] = true
			}
		}
		if next == since {
			return out, nil
		}
		since = next
	}
}
