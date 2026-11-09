package importer

import (
	"errors"
	"fmt"
	"net/url"
	"slices"
	"sort"
	"time"
)

const pageSize = 50

// ErrRefused is a token the importer will not use: an admin's, or an agent's.
var ErrRefused = errors.New("refused")

// Pod skip reasons, as the dry run prints them.
const (
	skipNotMember = "not-a-member"
	skipAdminRoom = "agent-admin"
	skipDM        = "dm-opt-in"
	skipNotOneOne = "dm-not-1-to-1"
	skipUnlisted  = "include-dm-not-listed"
)

// PodPlan is one pod the run will import: its messages still to post, in order, and what the run read around it.
type PodPlan struct {
	Pod     Pod
	Msgs    []Norm
	Already int
	Systems int
	Integ   []Integration
	Agents  []Member
}

// Plan is everything the run read and decided, before any write.
type Plan struct {
	Me      Me
	Pods    []*PodPlan
	Skipped map[string]int
	Grants  []Grant
}

// whoAmI refuses an admin or an agent before any pod is read. An admin's pod read would widen what canViewPod admits; a runtime
// token is not a person's.
func whoAmI(c *Commonly) (Me, error) {
	var me Me
	if err := c.Get("/api/auth/user", nil, &me); err != nil {
		return me, err
	}
	switch {
	case me.ID == "":
		return me, fmt.Errorf("%w: Commonly did not say who this token belongs to", ErrRefused)
	case me.Role == "admin":
		return me, fmt.Errorf("%w: this token belongs to an admin account; import with your own non-admin account, because an admin can read pods that are not theirs", ErrRefused)
	case me.IsBot:
		return me, fmt.Errorf("%w: this token belongs to an agent; import with your own user token", ErrRefused)
	}
	return me, nil
}

// classify decides a pod from the listing alone. Membership comes first: the person's own id must be in the member list, so the
// agent-dm fan-out and the admin bypass are never a reason to read.
func classify(p Pod, me Me, optedIn bool) string {
	switch {
	case !p.Has(me.ID):
		return skipNotMember
	case p.Type == "agent-admin":
		return skipAdminRoom
	case p.Type == "agent-room" || p.Type == "agent-dm":
		if !optedIn {
			return skipDM
		}
		if len(p.Members) != 2 {
			return skipNotOneOne
		}
	}
	return ""
}

// readPlan reads, one request at a time, and scrubs everything it will carry. Nothing is written anywhere.
func readPlan(c *Commonly, sc *Scrubber, o Options, st *State) (*Plan, error) {
	me, err := whoAmI(c)
	if err != nil {
		return nil, err
	}
	var listed []Pod
	if err := c.Get("/api/pods", nil, &listed); err != nil {
		return nil, err
	}
	plan := &Plan{Me: me, Skipped: map[string]int{}}
	include := map[string]bool{}
	for _, id := range o.IncludeDM {
		if id != "" {
			include[id] = true
		}
	}
	for _, p := range listed {
		optedIn := include[p.ID]
		delete(include, p.ID)
		if reason := classify(p, me, optedIn); reason != "" {
			plan.Skipped[reason]++
			continue
		}
		pp, err := readPod(c, sc, p, me, st)
		if err != nil {
			return nil, err
		}
		if pp != nil {
			plan.Pods = append(plan.Pods, pp)
		} else {
			plan.Skipped[skipNotMember]++
		}
	}
	if len(include) > 0 {
		plan.Skipped[skipUnlisted] = len(include)
	}
	return plan, readSides(c, sc, plan)
}

// readPod confirms membership against the pod itself, then reads its messages. A pod whose own member list does not hold the person is dropped.
func readPod(c *Commonly, sc *Scrubber, listed Pod, me Me, st *State) (*PodPlan, error) {
	var pod Pod
	if err := c.Get("/api/pods/"+url.PathEscape(listed.ID), nil, &pod); err != nil {
		var se *StatusError
		if errors.As(err, &se) {
			return nil, nil
		}
		return nil, err
	}
	pod.ID = listed.ID
	if !pod.Has(me.ID) {
		return nil, nil
	}
	if err := sc.Check("a pod name", pod.Name); err != nil {
		return nil, err
	}
	pp := &PodPlan{Pod: pod}
	msgs, err := readMessages(c, pod.ID, st.Pods[pod.ID])
	if err != nil {
		return nil, err
	}
	for _, n := range msgs {
		switch {
		case n.System:
			pp.Systems++
		case st.Pods[pod.ID].done(n.At, n.ID):
			pp.Already++
		default:
			if err := checkMessage(sc, n); err != nil {
				return nil, err
			}
			pp.Msgs = append(pp.Msgs, n)
		}
	}
	for _, m := range pod.Members {
		if m.IsBot {
			pp.Agents = append(pp.Agents, m)
		}
	}
	return pp, nil
}

func checkMessage(sc *Scrubber, n Norm) error {
	for field, v := range map[string]string{"a message body": n.Text, "an author name": n.Author, "a message id": n.ID, "a message time": n.At} {
		if err := sc.Check(field, v); err != nil {
			return err
		}
	}
	for _, f := range n.Files {
		if err := sc.Check("an attachment name", f); err != nil {
			return err
		}
	}
	return nil
}

// readMessages pages a pod newest to oldest, 50 at a time, and returns every message oldest first. The route's cursor is strict
// ("before"), so a page boundary inside a run of messages with one timestamp would drop the rest of the run: each next request asks
// for one millisecond past the oldest row, and the overlap is deduplicated by id. The read stops early once a page reaches below
// the saved cursor, because everything older was posted by an earlier run.
func readMessages(c *Commonly, podID string, ps *PodState) ([]Norm, error) {
	var all []Norm
	seen := map[string]bool{}
	before, lastBefore, strict := "", "", false
	for {
		q := url.Values{"limit": {fmt.Sprint(pageSize)}}
		if before != "" {
			q.Set("before", before)
		}
		var page []Message
		if err := c.Get("/api/messages/"+url.PathEscape(podID), q, &page); err != nil {
			return nil, err
		}
		fresh, oldest := 0, ""
		for _, m := range page {
			n := m.Normalize()
			if oldest == "" || n.At < oldest {
				oldest = n.At
			}
			if !seen[n.ID] {
				seen[n.ID] = true
				all = append(all, n)
				fresh++
			}
		}
		if len(page) < pageSize || oldest == "" || (ps != nil && ps.Cursor != "" && oldest < ps.Cursor) {
			break
		}
		strict = strict || fresh == 0
		next := oldest
		if t, err := time.Parse(timeLayout, oldest); err == nil && !strict {
			next = t.Add(time.Millisecond).Format(timeLayout)
		}
		if next == lastBefore {
			break
		}
		before, lastBefore = next, next
		strict = false
	}
	sort.SliceStable(all, func(i, j int) bool {
		if all[i].At != all[j].At {
			return all[i].At < all[j].At
		}
		return all[i].ID < all[j].ID
	})
	return all, nil
}

// readSides reads each imported pod's integrations and the person's own grants, through the allowlisted structs only.
func readSides(c *Commonly, sc *Scrubber, plan *Plan) error {
	for _, pp := range plan.Pods {
		var rows []Integration
		err := c.Get("/api/integrations/"+url.PathEscape(pp.Pod.ID), nil, &rows)
		var se *StatusError
		if err != nil && !errors.As(err, &se) {
			return err
		}
		pp.Integ = rows
	}
	var out struct {
		Grants []Grant `json:"grants"`
	}
	if err := c.Get("/api/grants", url.Values{"connectionOwner": {"me"}}, &out); err != nil {
		var se *StatusError
		if !errors.As(err, &se) {
			return err
		}
	}
	ids := map[string]bool{}
	for _, pp := range plan.Pods {
		ids[pp.Pod.ID] = true
	}
	for _, g := range out.Grants {
		if g.Target.Kind == "pod" && ids[g.Target.ID] && g.RevokedAt == nil {
			plan.Grants = append(plan.Grants, g)
		}
	}
	slices.SortStableFunc(plan.Grants, func(a, b Grant) int { return compareStr(a.Target.ID+a.ConnectorID(), b.Target.ID+b.ConnectorID()) })
	return nil
}

func compareStr(a, b string) int {
	switch {
	case a < b:
		return -1
	case a > b:
		return 1
	}
	return 0
}
