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
	"github.com/Team-Commonly/jellymoon/daemon/internal/workspace"
	"github.com/Team-Commonly/jellymoon/daemon/internal/ws"
)

// HeartbeatEvery is docs/protocol.md's interval: three missed beats make the pod call the computer offline.
const HeartbeatEvery = 15 * time.Second

// ReadTimeout is how long a pod socket may be silent before the daemon drops it and dials again: three heartbeat intervals, the
// same 45 s after which the pod marks the computer offline. The daemon pings with every heartbeat and the pod's pong (or any
// other frame) restarts the clock, so a healthy idle socket never trips it, and a half-open one is gone within a minute.
const ReadTimeout = 3 * HeartbeatEvery

// Daemon is one computer.
type Daemon struct {
	api   *api.Client
	store config.Store
	comp  config.Computer
	log   *log.Logger
	ws    *workspace.Manager // each run's working directory

	mu     sync.Mutex
	pods   map[protocol.PodID]bool
	waking map[protocol.WakeID]bool
	runs   map[protocol.RunID]context.CancelCauseFunc // the runs in progress, so run.stop can reach them
	wg     sync.WaitGroup
	ctx    context.Context
	beat   func(*ws.Conn, []byte) error // sends a heartbeat; a test seam, nil means Conn.Send

	readTimeout    time.Duration // how long a pod socket may stay silent; a test seam, zero means ReadTimeout
	heartbeatEvery time.Duration // the heartbeat interval; a test seam, zero means HeartbeatEvery
}

// New makes a daemon for a stored computer.
func New(a *api.Client, s config.Store, c config.Computer, l *log.Logger) *Daemon {
	return &Daemon{api: a, store: s, comp: c, log: l, ws: workspace.New(s.WorkspaceRoot()), pods: map[protocol.PodID]bool{}, waking: map[protocol.WakeID]bool{}, runs: map[protocol.RunID]context.CancelCauseFunc{}}
}

// Run keeps a socket open to every pod in the stored agent list until ctx ends, and serves `agent add` on the control socket.
func (d *Daemon) Run(ctx context.Context) error {
	d.ctx = ctx
	ctl, err := d.listen()
	if err != nil {
		return err
	}
	defer ctl.Close()
	// The control socket is held, so no other daemon runs on this config dir: every unkept run directory is from a dead one.
	if err := d.ws.Sweep(); err != nil {
		d.log.Printf("workspace sweep: %v", err)
	}
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
	d.log.Printf("stopped")
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
		if d.ctx.Err() != nil {
			return // a stop closed the socket: that is not a failure to report or retry
		}
		sleep, next := backoff(wait, time.Since(began), d.readTimeoutOrDefault())
		d.log.Printf("pod %s: socket ended (%v); reconnecting in %v", podID, err, sleep)
		select {
		case <-d.ctx.Done():
		case <-time.After(sleep):
		}
		wait = next
	}
}

// backoff is how long to wait before the next dial, and the wait after that. A connection that lived longer than one read
// timeout was healthy (it survived at least one silent stretch), so it starts the wait over at 1 s; otherwise the wait doubles,
// up to 30 s.
func backoff(wait, lived, readTimeout time.Duration) (sleep, next time.Duration) {
	if lived > readTimeout {
		wait = time.Second
	}
	return wait, min(wait*2, 30*time.Second)
}

func (d *Daemon) readTimeoutOrDefault() time.Duration {
	if d.readTimeout != 0 {
		return d.readTimeout
	}
	return ReadTimeout
}

// session is one connection: dial, heartbeat and ping at once and then every 15 s, and read until the socket fails or goes
// silent for ReadTimeout.
func (d *Daemon) session(podID protocol.PodID) error {
	conn, err := ws.Dial(d.ctx, d.api.PodSocket(podID), d.comp.Token)
	if err != nil {
		return err
	}
	defer conn.Close()
	conn.ReadTimeout = d.readTimeoutOrDefault()
	d.log.Printf("pod %s: connected", podID)
	ctx, stop := context.WithCancel(d.ctx)
	defer stop()
	go func() {
		<-ctx.Done()
		_ = conn.Close() // unblocks Read
	}()
	send := d.beat
	if send == nil {
		send = (*ws.Conn).Send
	}
	failed := make(chan error, 1)
	go func() {
		every := d.heartbeatEvery
		if every == 0 {
			every = HeartbeatEvery
		}
		t := time.NewTicker(every)
		defer t.Stop()
		for {
			err := send(conn, d.heartbeat(podID))
			if err == nil {
				err = conn.Ping() // its pong tells Read the path is still up
			}
			if err != nil {
				failed <- err
				stop() // closes conn, so the Read below returns and session reports this error
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
