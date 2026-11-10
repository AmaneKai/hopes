package gitsync

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

func gitIn(t *testing.T, dir string, args ...string) string {
	t.Helper()
	out, err := exec.Command("git", append([]string{"-C", dir}, args...)...).CombinedOutput()
	if err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, out)
	}
	return strings.TrimSpace(string(out))
}

func clone(t *testing.T, remote string) string {
	t.Helper()
	dir := t.TempDir()
	gitIn(t, dir, "clone", "-q", remote, ".")
	gitIn(t, dir, "config", "user.email", "test@example.com")
	gitIn(t, dir, "config", "user.name", "Test")
	return dir
}

func bareRemote(t *testing.T) string {
	t.Helper()
	remote := t.TempDir()
	gitIn(t, remote, "init", "-q", "--bare", "-b", "main")
	seed := clone(t, remote)
	writeItems(t, seed, "[]")
	gitIn(t, seed, "add", itemsFile)
	gitIn(t, seed, "commit", "-q", "-m", "seed")
	gitIn(t, seed, "push", "-q", "origin", "main")
	return remote
}

func writeItems(t *testing.T, dir, content string) {
	t.Helper()
	if err := os.WriteFile(filepath.Join(dir, itemsFile), []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func readItems(t *testing.T, dir string) string {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(dir, itemsFile))
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}

func commitAndPush(t *testing.T, dir, content string) {
	t.Helper()
	writeItems(t, dir, content)
	check(t, StageAndCommit(dir, "sync: update tasks"))
	check(t, Push(dir, "origin", "main"))
}

func check(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func commits(t *testing.T, remote string) int {
	n, _ := strconv.Atoi(gitIn(t, remote, "rev-list", "--count", "main"))
	return n
}

func TestIsRepoDetectsDotGit(t *testing.T) {
	dir := t.TempDir()
	if IsRepo(dir) {
		t.Fatal("empty dir is not a repo")
	}
	gitIn(t, dir, "init", "-q")
	if !IsRepo(dir) {
		t.Fatal("should be a repo")
	}
}

func TestDetectsUncommittedAndUnpushedChanges(t *testing.T) {
	work := clone(t, bareRemote(t))
	steps := []struct {
		do   func()
		want bool
	}{
		{func() {}, false},
		{func() { writeItems(t, work, `[{"title":"a"}]`) }, true},
		{func() { check(t, StageAndCommit(work, "m")) }, true},
		{func() { check(t, Push(work, "origin", "main")) }, false},
	}
	for i, step := range steps {
		step.do()
		if HasUnsyncedChanges(work, "origin", "main") != step.want {
			t.Fatalf("step %d: want %v", i, step.want)
		}
	}
}

func TestCommitPushAndPullRoundTrip(t *testing.T) {
	remote := bareRemote(t)
	a, b := clone(t, remote), clone(t, remote)
	check(t, StageAndCommit(a, "nothing changed"))
	commitAndPush(t, b, `[{"title":"from-b"}]`)
	check(t, PullRebase(a, "origin", "main"))
	if readItems(t, a) != `[{"title":"from-b"}]` {
		t.Fatal("pull did not bring remote changes")
	}
}

func TestConflictsAreDetectedAndResolvedKeepingRemote(t *testing.T) {
	remote := bareRemote(t)
	a, b := clone(t, remote), clone(t, remote)
	commitAndPush(t, b, `[{"title":"from-b"}]`)
	writeItems(t, a, `[{"title":"from-a"}]`)
	check(t, StageAndCommit(a, "m"))
	if !errors.Is(Push(a, "origin", "main"), ErrRejected) {
		t.Fatal("push should be rejected")
	}
	if !errors.Is(PullRebase(a, "origin", "main"), ErrConflict) {
		t.Fatal("pull should conflict")
	}
	backup, err := ResolveConflictKeepRemote(a, "origin", "main")
	check(t, err)
	if readFile(t, backup) != `[{"title":"from-a"}]` {
		t.Fatal("backup should hold the local version")
	}
	if readItems(t, a) != `[{"title":"from-b"}]` {
		t.Fatal("working copy should match the remote")
	}
}

func readFile(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	check(t, err)
	return string(data)
}

func waitFor(t *testing.T, what string, done func() bool) {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for !done() {
		if time.Now().After(deadline) {
			t.Fatal(what + " never happened")
		}
		time.Sleep(50 * time.Millisecond)
	}
}

func spawn(dir string, debounce time.Duration) *Engine {
	opts := Options{Dir: dir, Remote: "origin", Branch: "main", Debounce: debounce, Enabled: true}
	return Spawn(opts)
}

func TestRapidEditsCollapseIntoASingleCommit(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	before := commits(t, remote)
	engine := spawn(work, 300*time.Millisecond)
	defer engine.Shutdown()
	for i := range 5 {
		writeItems(t, work, `[{"title":"edit-`+strconv.Itoa(i)+`"}]`)
		engine.RequestPush()
		time.Sleep(50 * time.Millisecond)
	}
	waitFor(t, "the push", func() bool { return commits(t, remote) > before })
	if commits(t, remote) != before+1 {
		t.Fatal("edits should collapse into one commit")
	}
	if readItems(t, clone(t, remote)) != `[{"title":"edit-4"}]` {
		t.Fatal("the last edit should win")
	}
}

func TestFlushPushesUncommittedEdits(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	before := commits(t, remote)
	writeItems(t, work, `[{"title":"quit-time"}]`)
	Flush(work, "origin", "main")
	if commits(t, remote) != before+1 {
		t.Fatal("flush should push")
	}
}

func TestEnginePushesLeftoverChangesOnStartup(t *testing.T) {
	remote := bareRemote(t)
	work := clone(t, remote)
	before := commits(t, remote)
	writeItems(t, work, `[{"title":"left-behind"}]`)
	engine := spawn(work, 100*time.Millisecond)
	defer engine.Shutdown()
	waitFor(t, "the leftover push", func() bool { return commits(t, remote) > before })
}

func TestDisabledWhenTheDirectoryIsNotARepo(t *testing.T) {
	engine := spawn(t.TempDir(), 100*time.Millisecond)
	if engine.Enabled || (<-engine.Statuses()).Kind != Disabled {
		t.Fatal("sync should be disabled")
	}
}
