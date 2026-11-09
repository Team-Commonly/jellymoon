package importer

import (
	"net/http"
	"sort"
	"strconv"
	"sync"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// fakeSink is Jellymoon: it records what is written, and can lose the acknowledgement of one post (it lands, the caller sees an error).
type fakeSink struct {
	mu      sync.Mutex
	pods    map[string]string // name -> id
	posts   map[string][]protocol.PostInput
	writes  int
	lostAck int // the nth post lands but returns an error; 0 means never
}

func newSink() *fakeSink {
	return &fakeSink{pods: map[string]string{}, posts: map[string][]protocol.PostInput{}}
}

func (s *fakeSink) CreatePod(name string) (string, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.writes++
	id := "pod_" + strconv.Itoa(len(s.pods)+1)
	s.pods[name] = id
	return id, nil
}

func (s *fakeSink) Post(podID string, in protocol.PostInput) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	total := 0
	for _, p := range s.posts {
		total += len(p)
	}
	s.writes++
	s.posts[podID] = append(s.posts[podID], in)
	if s.lostAck > 0 && total+1 == s.lostAck {
		return http.ErrHandlerTimeout
	}
	return nil
}

func (s *fakeSink) Sources(podID string) (map[string]bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := map[string]bool{}
	for _, p := range s.posts[podID] {
		if p.Body != nil {
			out[markerOf(*p.Body)] = true
		}
	}
	return out, nil
}

// bodies lists every posted body of every pod, in posting order per pod.
func (s *fakeSink) bodies() []string {
	s.mu.Lock()
	defer s.mu.Unlock()
	var out []string
	for _, id := range s.sortedPods() {
		for _, p := range s.posts[id] {
			out = append(out, *p.Body)
		}
	}
	return out
}

func (s *fakeSink) sortedPods() []string {
	var ids []string
	for id := range s.posts {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return ids
}
