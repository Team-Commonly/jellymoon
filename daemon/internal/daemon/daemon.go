// Package daemon is the long-running process: it holds the computer's token, keeps a socket open to every pod it hosts an
// agent in, and tells each pod it is alive every 15 s. The pod derives presence from that socket and those heartbeats.
package daemon

import (
	"context"
	"encoding/json"
	"log"
	"sync"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/ws"
)

// HeartbeatEvery is docs/protocol.md's interval: three missed beats make the pod call the computer offline.
const HeartbeatEvery = 15 * time.Second

// Daemon is one computer.
type Daemon struct {
	api   *api.Client
	store config.Store
	comp  config.Computer
	log   *log.Logger

	mu     sync.Mutex
	pods   map[protocol.PodID]bool
	waking map[protocol.WakeID]bool
	wg     sync.WaitGroup
	ctx    context.Context
}

// New makes a daemon for a stored computer.
func New(a *api.Client, s config.Store, c config.Computer, l *log.Logger) *Daemon {
	return &Daemon{api: a, store: s, comp: c, log: l, pods: map[protocol.PodID]bool{}, waking: map[protocol.WakeID]bool{}}
}

// Run keeps a socket open to every pod in the stored agent list until ctx ends, and serves `agent add` on the control socket.
func (d *Daemon) Run(ctx context.Context) error {
	d.ctx = ctx
	ctl, err := d.listen()
	if err != nil {
		return err
	}
	defer ctl.Close()
	agents, err := d.store.Agents()
	if err != nil {
		return err
	}
	for _, a := range agents {
		d.watch(a.PodID)
	}
	go d.serve(ctl)
	<-ctx.Done()
	d.wg.Wait()
	return nil
}

// watch starts the pod's socket loop once.
func (d *Daemon) watch(podID protocol.PodID) {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.pods[podID] {
		return
	}
	d.pods[podID] = true
	d.wg.Add(1)
	go func() {
		defer d.wg.Done()
		d.keep(podID)
	}()
}

// keep connects to the pod and reconnects after a failure, waiting 1 s, then twice as long each time up to 30 s.
func (d *Daemon) keep(podID protocol.PodID) {
	wait := time.Second
	for d.ctx.Err() == nil {
		began := time.Now()
		err := d.session(podID)
		if time.Since(began) > time.Minute {
			wait = time.Second
		}
		d.log.Printf("pod %s: socket ended (%v); reconnecting in %v", podID, err, wait)
		select {
		case <-d.ctx.Done():
		case <-time.After(wait):
		}
		wait = min(wait*2, 30*time.Second)
	}
}

// session is one connection: dial, heartbeat at once and then every 15 s, and read until the socket fails.
func (d *Daemon) session(podID protocol.PodID) error {
	conn, err := ws.Dial(d.ctx, d.api.PodSocket(podID), d.comp.Token)
	if err != nil {
		return err
	}
	defer conn.Close()
	d.log.Printf("pod %s: connected", podID)
	ctx, stop := context.WithCancel(d.ctx)
	defer stop()
	go func() {
		<-ctx.Done()
		_ = conn.Close() // unblocks Read
	}()
	failed := make(chan error, 1)
	go func() {
		t := time.NewTicker(HeartbeatEvery)
		defer t.Stop()
		for {
			if err := conn.Send(d.heartbeat(podID)); err != nil {
				failed <- err
				return
			}
			select {
			case <-ctx.Done():
				return
			case <-t.C:
			}
		}
	}()
	for {
		raw, err := conn.Read()
		if err == nil {
			d.onFrame(conn, raw)
		} else {
			select {
			case werr := <-failed:
				return werr
			default:
				return err
			}
		}
	}
}

func (d *Daemon) heartbeat(podID protocol.PodID) []byte {
	data, _ := json.Marshal(protocol.PresenceHeartbeat{ComputerID: &d.comp.ComputerID})
	b, _ := json.Marshal(protocol.Envelope{V: 1, Type: protocol.EventTypePresenceHeartbeat, PodID: podID, Data: data})
	return b
}
