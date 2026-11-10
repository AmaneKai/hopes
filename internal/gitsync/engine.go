package gitsync

import (
	"errors"
	"os"
	"os/exec"
	"sync"
	"sync/atomic"
	"time"

	"github.com/AmaneKai/hopes/internal/proc"
)

const FlushArg = "--sync-flush"

type Kind int

const (
	Disabled Kind = iota
	Idle
	Syncing
	Synced
	UpdatedRemoteData
	Offline
	Conflict
	Failed
)

type Status struct {
	Kind   Kind
	Detail string
}

type request int

const (
	schedulePush request = iota
	forceSync
)

const shutdownWait = 30 * time.Second

type Engine struct {
	Enabled  bool
	requests chan request
	statuses chan Status
	dir      string
	remote   string
	branch   string
	dirty    atomic.Bool
	done     chan struct{}
	finished chan struct{}
	stopOnce sync.Once
}

type Options struct {
	Dir           string
	Remote        string
	Branch        string
	Debounce      time.Duration
	PullOnStartup bool
	Enabled       bool
}

func Spawn(o Options) *Engine {
	e := &Engine{
		requests: make(chan request, 64),
		statuses: make(chan Status, 256),
		done:     make(chan struct{}),
		finished: make(chan struct{}),
		dir:      o.Dir,
		remote:   o.Remote,
		branch:   o.Branch,
	}
	if !o.Enabled || !IsRepo(o.Dir) {
		e.statuses <- Status{Kind: Disabled}
		return e
	}
	e.Enabled = true
	w := &worker{engine: e, debounce: o.Debounce}
	go w.loop(o.PullOnStartup)
	return e
}

func (e *Engine) Statuses() <-chan Status { return e.statuses }

func (e *Engine) RequestPush() {
	if e.Enabled {
		e.dirty.Store(true)
		e.enqueue(schedulePush)
	}
}

func (e *Engine) ForceSync() {
	if e.Enabled {
		e.enqueue(forceSync)
	}
}

func (e *Engine) enqueue(r request) {
	select {
	case e.requests <- r:
	default:
	}
}

func (e *Engine) Shutdown() {
	if !e.Enabled {
		return
	}
	e.stopOnce.Do(func() { close(e.done) })
	select {
	case <-e.finished:
	case <-time.After(shutdownWait):
		return
	}
	if !e.dirty.Load() {
		return
	}
	exe, err := os.Executable()
	if err != nil {
		return
	}
	cmd := exec.Command(exe, FlushArg, e.dir, e.remote, e.branch)
	proc.Detach(cmd)
	_ = cmd.Start()
}

func Flush(dir, remote, branch string) Status {
	e := &Engine{statuses: make(chan Status, 64), dir: dir, remote: remote, branch: branch}
	w := &worker{engine: e}
	w.push()
	last := Status{Kind: Idle}
	for {
		select {
		case st := <-e.statuses:
			last = st
		default:
			return last
		}
	}
}

type worker struct {
	engine   *Engine
	debounce time.Duration
}

func (w *worker) report(kind Kind, detail string) {
	if kind == Synced {
		w.engine.dirty.Store(false)
	}
	select {
	case w.engine.statuses <- Status{Kind: kind, Detail: detail}:
	default:
	}
}

func (w *worker) reportErr(err error) {
	switch {
	case errors.Is(err, ErrOffline):
		w.report(Offline, "")
	case errors.Is(err, ErrConflict):
		w.resolveConflict()
	default:
		w.report(Failed, err.Error())
	}
}

func (w *worker) loop(pullOnStartup bool) {
	defer close(w.engine.finished)
	if pullOnStartup {
		w.pull()
	} else {
		w.report(Idle, "")
	}
	var deadline <-chan time.Time
	e := w.engine
	if HasUnsyncedChanges(e.dir, e.remote, e.branch) {
		w.report(Syncing, "Pending sync...")
		deadline = time.After(w.debounce)
	}
	for {
		select {
		case <-e.done:
			return
		case req := <-e.requests:
			deadline = w.handle(req)
		case <-deadline:
			deadline = w.retryLater(w.push())
		}
	}
}

func (w *worker) handle(req request) <-chan time.Time {
	if req == forceSync {
		return w.retryLater(w.push())
	}
	w.report(Syncing, "Pending sync...")
	return time.After(w.debounce)
}

func (w *worker) retryLater(retry bool) <-chan time.Time {
	if retry {
		return time.After(w.debounce)
	}
	return nil
}

func (w *worker) withRepoLock(action func()) (acquired bool) {
	unlock, err := Lock(w.engine.dir)
	if err != nil {
		w.report(Failed, err.Error())
		return false
	}
	defer unlock()
	action()
	return true
}

func (w *worker) pull() {
	w.withRepoLock(w.pullLocked)
}

func (w *worker) pullLocked() {
	e := w.engine
	w.report(Syncing, "Pulling...")
	if err := PullRebase(e.dir, e.remote, e.branch); err != nil {
		w.reportErr(err)
		return
	}
	w.report(UpdatedRemoteData, "")
}

func (w *worker) resolveConflict() {
	e := w.engine
	backup, err := ResolveConflictKeepRemote(e.dir, e.remote, e.branch)
	if err != nil {
		w.report(Failed, err.Error())
		return
	}
	w.report(Conflict, backup)
}

func (w *worker) push() (retry bool) {
	acquired := w.withRepoLock(func() { retry = w.pushLocked() })
	return retry || !acquired
}

func (w *worker) pushLocked() bool {
	e := w.engine
	w.report(Syncing, "Committing...")
	if err := StageAndCommit(e.dir, "sync: update tasks"); err != nil {
		w.report(Failed, err.Error())
		return false
	}
	w.report(Syncing, "Pushing...")
	err := Push(e.dir, e.remote, e.branch)
	if errors.Is(err, ErrRejected) {
		return w.rebaseAndPush()
	}
	return w.pushOutcome(err)
}

func (w *worker) rebaseAndPush() bool {
	e := w.engine
	w.report(Syncing, "Pulling...")
	err := PullRebase(e.dir, e.remote, e.branch)
	if errors.Is(err, ErrConflict) {
		w.resolveConflict()
		return false
	}
	if err != nil {
		return w.pushOutcome(err)
	}
	w.report(UpdatedRemoteData, "")
	return w.pushOutcome(Push(e.dir, e.remote, e.branch))
}

func (w *worker) pushOutcome(err error) bool {
	if err == nil {
		w.report(Synced, "")
		return false
	}
	w.reportErr(err)
	return errors.Is(err, ErrOffline)
}
