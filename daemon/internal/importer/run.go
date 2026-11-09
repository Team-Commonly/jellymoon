package importer

import (
	"cmp"
	"crypto/sha256"
	"fmt"
	"sort"
	"strings"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// Sink is Jellymoon, as the importer uses it: the existing create-pod and post routes, under the person's own jmu_ token, and a
// read of a pod's feed to find a post whose answer was lost. It is nil in a dry run.
type Sink interface {
	CreatePod(name string) (string, error)
	Post(podID string, in protocol.PostInput) error
	Sources(podID string) (map[string]bool, error)
}

// Options are the command's flags.
type Options struct {
	Apply     bool
	IncludeDM []string
	Anonymize bool
	StatePath string
	OutDir    string
	Server    string // the Jellymoon server, kept in the state file so another server's state is never reused
}

// Report is what the run counted.
type Report struct {
	Apply             bool
	PodsImported      int
	PodsSkipped       map[string]int
	Messages          int
	MessagesPosted    int
	MessagesDone      int
	Cards             int
	Grants            int
	GrantsUnsupported int
	Integrations      int
	OutDir            string
}

func (r *Report) String() string {
	var b strings.Builder
	if r.Apply {
		b.WriteString("applied\n")
	} else {
		b.WriteString("dry run: nothing was written (add --apply to import)\n")
	}
	fmt.Fprintf(&b, "pods: %d to import\n", r.PodsImported)
	if len(r.PodsSkipped) > 0 {
		var parts []string
		for k, n := range r.PodsSkipped {
			parts = append(parts, fmt.Sprintf("%s: %d", k, n))
		}
		sort.Strings(parts)
		fmt.Fprintf(&b, "skipped pods: %s\n", strings.Join(parts, ", "))
	}
	fmt.Fprintf(&b, "messages: %d to post, %d already imported", r.Messages, r.MessagesDone)
	if r.Apply {
		fmt.Fprintf(&b, ", %d posted now", r.MessagesPosted)
	}
	fmt.Fprintf(&b, "\nagent identity cards: %d\ngrants on the checklist: %d (%d unsupported)\nintegrations to reconnect: %d\n", r.Cards, r.Grants, r.GrantsUnsupported, r.Integrations)
	if r.Apply {
		fmt.Fprintf(&b, "cards and checklist written to %s\n", r.OutDir)
	}
	return b.String()
}

// post is one message ready to send: scrubbed, and composed with its prefix and marker.
type post struct {
	n  Norm
	in protocol.PostInput
}

type podWork struct {
	pp    *PodPlan
	name  string
	posts []post
}

// Run reads everything and checks it first. If any string it would copy looks like a credential, it fails before the first write
// to Jellymoon and before any local file is made, so no earlier pod stands. Only then does --apply write, one pod at a time,
// saving the state after every post.
func Run(c *Commonly, sink Sink, sc *Scrubber, o Options) (*Report, error) {
	st, err := LoadState(o.StatePath, c.Host(), o.Server)
	if err != nil {
		return nil, err
	}
	plan, err := readPlan(c, sc, o, st)
	if err != nil {
		return nil, err
	}
	work, err := prepare(plan, st, sc, o)
	if err != nil {
		return nil, err
	}
	files, err := sideFiles(plan, sc)
	if err != nil {
		return nil, err
	}
	rep := summarize(plan, work, files, o)
	if !o.Apply {
		return rep, nil
	}
	return rep, apply(sink, st, work, files, o, rep)
}

func summarize(plan *Plan, work []podWork, f *sideOut, o Options) *Report {
	rep := &Report{Apply: o.Apply, PodsImported: len(work), PodsSkipped: plan.Skipped, OutDir: o.OutDir,
		Cards: len(f.cards), Grants: f.grants, GrantsUnsupported: f.unsupported, Integrations: f.integrations}
	for _, w := range work {
		rep.Messages += len(w.posts)
		rep.MessagesDone += w.pp.Already
	}
	return rep
}

// prepare composes and scrubs every post. Authors that are people take a stable "participant X" under --anonymize; agents keep their name.
func prepare(plan *Plan, st *State, sc *Scrubber, o Options) ([]podWork, error) {
	var out []podWork
	for _, pp := range plan.Pods {
		w := podWork{pp: pp, name: podName(pp.Pod.Name)}
		bots := map[string]bool{}
		for _, m := range pp.Agents {
			bots[m.ID] = true
		}
		for _, n := range pp.Msgs {
			name := oneLine(n.Author)
			if o.Anonymize && !n.AuthorBot && !bots[n.AuthorID] {
				name = anonName(st, cmp.Or(n.AuthorID, nameKey(n.Author)))
			}
			in := compose(n, name)
			for field, v := range map[string]string{"a summary": in.Summary, "a body": *in.Body} {
				if err := sc.Check(field, v); err != nil {
					return nil, fmt.Errorf("%w (%s)", err, where(sc, pp.Pod.ID, n.ID))
				}
			}
			w.posts = append(w.posts, post{n: n, in: in})
		}
		out = append(out, w)
	}
	return out, nil
}

// nameKey stands in for an author id a message does not carry, so the state file keeps a hash and never a name.
func nameKey(name string) string { return fmt.Sprintf("name:%x", sha256.Sum256([]byte(name)))[:17] }

func anonName(st *State, key string) string {
	if v, ok := st.Anon[key]; ok {
		return v
	}
	v := participant(len(st.Anon))
	st.Anon[key] = v
	return v
}

// apply creates each pod once and posts its messages oldest first. A post that may have landed without its answer is found again by its marker.
func apply(sink Sink, st *State, work []podWork, f *sideOut, o Options, rep *Report) error {
	if err := st.Save(o.StatePath); err != nil {
		return err
	}
	for _, w := range work {
		ps := st.Pods[w.pp.Pod.ID]
		if ps == nil {
			id, err := sink.CreatePod(w.name)
			if err != nil {
				return fmt.Errorf("creating a pod: %w", err)
			}
			ps = &PodState{JM: id}
			st.Pods[w.pp.Pod.ID] = ps
			if err := st.Save(o.StatePath); err != nil {
				return err
			}
		}
		f.jm[w.pp.Pod.ID] = ps.JM
		if err := postAll(sink, st, ps, w, o, rep); err != nil {
			return err
		}
	}
	return f.write(o.OutDir)
}

func postAll(sink Sink, st *State, ps *PodState, w podWork, o Options, rep *Report) error {
	var landed map[string]bool
	for _, p := range w.posts {
		if ps.Inflight == p.n.ID && landed == nil {
			found, err := sink.Sources(ps.JM)
			if err != nil {
				return fmt.Errorf("checking the pod for the last post: %w", err)
			}
			landed = found
		}
		if ps.Inflight == p.n.ID && landed[sourceMarker(p.n.ID)] {
			ps.advance(p.n.At, p.n.ID)
			rep.MessagesDone++
		} else {
			ps.Inflight = p.n.ID
			if err := st.Save(o.StatePath); err != nil {
				return err
			}
			if err := sink.Post(ps.JM, p.in); err != nil {
				return fmt.Errorf("posting a message: %w", err)
			}
			ps.advance(p.n.At, p.n.ID)
			rep.MessagesPosted++
		}
		if err := st.Save(o.StatePath); err != nil {
			return err
		}
	}
	return nil
}
