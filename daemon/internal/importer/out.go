package importer

import (
	"encoding/json"
	"fmt"
	"path/filepath"
	"sort"
	"strings"
)

// Card is an agent's identity as the person can feed it to `jellymoon agent add`: a name and a look. It holds no memory, no
// runtime, no token and no harness; the person picks the harness when they add the agent.
type Card struct {
	DisplayName string   `json:"displayName"`
	Handle      string   `json:"handle,omitempty"`
	Instance    string   `json:"instance,omitempty"`
	Avatar      string   `json:"avatar,omitempty"`
	Pods        []string `json:"pods"`
}

// sideOut is the cards and the checklist, already scrubbed, with the counts the report shows.
type sideOut struct {
	cards        []Card
	checklist    []string
	grants       int
	unsupported  int
	integrations int
	jm           map[string]string // commonly pod id -> jellymoon pod id, filled in as pods are created
	podNames     map[string]string
}

// supported are the connector vendors Jellymoon has (schema ConnectorVendor). The rest are listed as unsupported.
var supported = map[string]bool{"linear": true}

// sideFiles builds the cards and the checklist from the allowlisted structs and scrubs every string in them.
func sideFiles(plan *Plan, sc *Scrubber) (*sideOut, error) {
	f := &sideOut{jm: map[string]string{}, podNames: map[string]string{}}
	seen := map[string]bool{}
	for _, pp := range plan.Pods {
		f.podNames[pp.Pod.ID] = podName(pp.Pod.Name)
		for _, m := range pp.Agents {
			if !seen[m.ID] {
				seen[m.ID] = true
				f.cards = append(f.cards, Card{DisplayName: oneLine(m.Label()), Handle: oneLine(m.Username), Instance: oneLine(m.InstanceID), Avatar: oneLine(m.Avatar)})
			}
		}
	}
	for i := range f.cards {
		f.cards[i].Pods = podsOf(plan, f.cards[i].DisplayName)
	}
	for _, g := range plan.Grants {
		line := grantLine(g, f.podNames[g.Target.ID])
		f.checklist = append(f.checklist, line)
		f.grants++
		if !supported[g.ConnectorID()] {
			f.unsupported++
		}
	}
	for _, pp := range plan.Pods {
		for _, it := range pp.Integ {
			f.checklist = append(f.checklist, fmt.Sprintf("- [ ] integration %s (%s) on pod %q: reconnect it by hand, Jellymoon cannot import it", oneLine(it.Type), oneLine(it.Status), f.podNames[pp.Pod.ID]))
			f.integrations++
		}
	}
	return f, f.scrub(sc)
}

func podsOf(plan *Plan, label string) []string {
	var out []string
	for _, pp := range plan.Pods {
		for _, m := range pp.Agents {
			if oneLine(m.Label()) == label {
				out = append(out, podName(pp.Pod.Name))
				break
			}
		}
	}
	return out
}

func grantLine(g Grant, pod string) string {
	access := "write"
	if g.WriteMode == "read" {
		access = "read"
	}
	name := oneLine(g.ConnectorID())
	if name == "" {
		name = "unknown"
	}
	if !supported[name] {
		return fmt.Sprintf("- [ ] %s, %s, pod %q: unsupported, Jellymoon connects linear only", name, access, pod)
	}
	return fmt.Sprintf("- [ ] %s, %s, pod %q: connect it again in Jellymoon (paste a token there), then grant the agent access", name, access, pod)
}

func (f *sideOut) scrub(sc *Scrubber) error {
	for _, c := range f.cards {
		raw, _ := json.Marshal(c)
		if err := sc.Check("an agent card", string(raw)); err != nil {
			return err
		}
	}
	return sc.Check("the checklist", strings.Join(f.checklist, "\n"))
}

// write makes the two files, 0600 in a 0700 directory. The checklist names the Jellymoon pod each Commonly pod became.
func (f *sideOut) write(dir string) error {
	raw, err := json.MarshalIndent(f.cards, "", "  ")
	if err != nil {
		return err
	}
	if err := writeOwnerOnly(filepath.Join(dir, "agent-cards.json"), append(raw, '\n')); err != nil {
		return err
	}
	var b strings.Builder
	b.WriteString("# Re-authorisation checklist\n\nGrants and integrations do not import: a credential never travels. Do each by hand in Jellymoon.\n\n")
	if len(f.checklist) == 0 {
		b.WriteString("Nothing to redo.\n")
	}
	for _, l := range f.checklist {
		b.WriteString(l + "\n")
	}
	b.WriteString("\nImported pods:\n")
	ids := make([]string, 0, len(f.jm))
	for cm := range f.jm {
		ids = append(ids, cm)
	}
	sort.Strings(ids)
	for _, cm := range ids {
		fmt.Fprintf(&b, "- %q is %s in Jellymoon\n", f.podNames[cm], f.jm[cm])
	}
	return writeOwnerOnly(filepath.Join(dir, "grants-checklist.md"), []byte(b.String()))
}
