package daemon

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/http/httptest"
	"sync"
	"sync/atomic"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// Agents added at the same moment all survive a reload: the list is read, changed and written back as one step, so no
// add overwrites another's, and after a restart every agent is still watched.
func TestConcurrentAddsAllSurviveAReload(t *testing.T) {
	var n atomic.Int64
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = io.Copy(io.Discard, r.Body)
		_ = json.NewEncoder(w).Encode(protocol.RegisterAgentResponse{AgentID: fmt.Sprintf("agt_%d", n.Add(1))})
	}))
	defer srv.Close()
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	store := config.Store{Dir: t.TempDir()}
	d := New(c, store, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // watch starts no session on an ended daemon
	d.ctx = ctx
	const adds = 24
	var wg sync.WaitGroup
	for i := 0; i < adds; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if _, err := d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessFake, DisplayName: "a"}); err != nil {
				t.Error(err)
			}
		}()
	}
	wg.Wait()
	d.wg.Wait()
	got, err := store.Agents()
	if err != nil {
		t.Fatal(err)
	}
	if len(got) != adds {
		t.Errorf("%d agents survive a reload, want %d", len(got), adds)
	}
}
