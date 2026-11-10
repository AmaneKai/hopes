package gitsync

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func drain(e *Engine) (last Status) {
	for {
		select {
		case st := <-e.Statuses():
			last = st
		default:
			return last
		}
	}
}

func hasStatus(e *Engine, kind Kind, seen *[]Kind) bool {
	for {
		select {
		case st := <-e.Statuses():
			*seen = append(*seen, st.Kind)
			if st.Kind == kind {
				return true
			}
		default:
			return false
		}
	}
}

func TestTwoEnginesConvergeAfterOneEdits(t *testing.T) {
	remote := bareRemote(t)
	a, b := clone(t, remote), clone(t, remote)
	engineA := spawn(a, 100*time.Millisecond)
	defer engineA.Shutdown()
	writeItems(t, a, `[{"title":"from-a"}]`)
	engineA.RequestPush()
	waitFor(t, "a's push", func() bool { return readItems(t, clone(t, remote)) == `[{"title":"from-a"}]` })

	engineB := Spawn(Options{Dir: b, Remote: "origin", Branch: "main", Debounce: 100 * time.Millisecond, PullOnStartup: true, Enabled: true})
	defer engineB.Shutdown()
	var seen []Kind
	waitFor(t, "b's pull", func() bool { return hasStatus(engineB, UpdatedRemoteData, &seen) })
	if readItems(t, b) != `[{"title":"from-a"}]` {
		t.Fatalf("b did not receive a's edit, statuses: %v", seen)
	}
}

func TestConflictingEnginesReportConflictAndKeepRemote(t *testing.T) {
	remote := bareRemote(t)
	a, b := clone(t, remote), clone(t, remote)
	commitAndPush(t, a, `[{"title":"from-a"}]`)
	engineB := spawn(b, 50*time.Millisecond)
	defer engineB.Shutdown()
	drain(engineB)
	writeItems(t, b, `[{"title":"from-b"}]`)
	engineB.RequestPush()
	var seen []Kind
	waitFor(t, "the conflict report", func() bool { return hasStatus(engineB, Conflict, &seen) })
	if readItems(t, b) != `[{"title":"from-a"}]` {
		t.Fatalf("working copy should match the remote, statuses: %v", seen)
	}
	backups, _ := filepath.Glob(filepath.Join(b, itemsFile+".local-backup-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]"))
	if len(backups) != 1 || readFile(t, backups[0]) != `[{"title":"from-b"}]` {
		t.Fatalf("expected one backup holding the local edit, got %v", backups)
	}
	if readFile(t, BasePathFor(backups[0])) != `[]` {
		t.Fatal("the merge base should be the seed commit's contents")
	}
}

func TestShutdownReturnsPromptlyAndReleasesTheLock(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	engine := spawn(work, 50*time.Millisecond)
	done := make(chan struct{})
	go func() {
		engine.Shutdown()
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(5 * time.Second):
		t.Fatal("Shutdown hung")
	}
	if _, err := os.Stat(filepath.Join(work, ".git", lockName)); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("lock file left behind: %v", err)
	}
}

func TestMisconfiguredRemoteIsAnErrorNotOffline(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	gitIn(t, work, "remote", "set-url", "origin", filepath.Join(t.TempDir(), "missing.git"))
	err := Push(work, "origin", "main")
	if err == nil || errors.Is(err, ErrOffline) {
		t.Fatalf("a missing remote must be a real error, got %v", err)
	}
}

func TestRequestsNeverBlockWhenTheWorkerIsBusy(t *testing.T) {
	engine := &Engine{Enabled: true, requests: make(chan request, 1)}
	finished := make(chan struct{})
	go func() {
		for range 1000 {
			engine.RequestPush()
			engine.ForceSync()
		}
		close(finished)
	}()
	select {
	case <-finished:
	case <-time.After(2 * time.Second):
		t.Fatal("RequestPush blocked on a full queue")
	}
}

func TestMain(m *testing.M) {
	if len(os.Args) == 5 && os.Args[1] == FlushArg {
		Flush(os.Args[2], os.Args[3], os.Args[4])
		return
	}
	os.Exit(m.Run())
}

func TestShutdownWithPendingEditsFlushesThroughADetachedProcess(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	before := commits(t, remote)
	engine := spawn(work, time.Hour)
	writeItems(t, work, `[{"title":"typed-then-quit"}]`)
	engine.RequestPush()
	engine.Shutdown()
	waitFor(t, "the detached flush", func() bool { return commits(t, remote) > before })
	if readItems(t, clone(t, remote)) != `[{"title":"typed-then-quit"}]` {
		t.Fatal("flush pushed the wrong content")
	}
}
