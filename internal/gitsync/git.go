package gitsync

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/AmaneKai/hopes/internal/proc"
)

const (
	networkTimeout = 10 * time.Second
	outputLimit    = 1 << 20
	itemsFile      = "items.json"
)

var (
	ErrOffline     = errors.New("network unreachable")
	ErrConflict    = errors.New("merge conflict")
	ErrRejected    = errors.New("remote has newer commits")
	ErrLocked      = errors.New("another hopes sync is running")
	offlineMarkers = []string{
		"could not resolve host",
		"failed to connect",
		"connection timed out",
		"connection refused",
		"network is unreachable",
		"could not read from remote repository",
		"no route to host",
	}
	authMarkers = []string{
		"authentication failed",
		"permission denied",
		"could not read username",
		"could not read password",
		"repository not found",
		"invalid credentials",
		"returned error: 403",
		"returned error: 404",
		"does not appear to be a git repository",
		"does not exist",
		"no such remote",
	}
	rejectedMarkers = []string{"[rejected]", "non-fast-forward", "fetch first"}
	conflictMarkers = []string{"CONFLICT", "could not apply"}
)

func IsRepo(dir string) bool {
	_, err := os.Stat(filepath.Join(dir, ".git"))
	return err == nil
}

var gitEnv = []string{"LC_ALL=C", "LANG=C", "GIT_TERMINAL_PROMPT=0"}

func runGit(timeout time.Duration, dir string, args ...string) (proc.Result, error) {
	res, err := proc.RunEnv("git", args, dir, gitEnv, nil, outputLimit, timeout)
	if err != nil {
		return res, err
	}
	if res.TimedOut {
		return res, ErrOffline
	}
	return res, nil
}

func git(dir string, args ...string) (proc.Result, error) {
	return runGit(0, dir, args...)
}

func gitNet(dir string, args ...string) (proc.Result, error) {
	return runGit(networkTimeout, dir, args...)
}

func containsAny(text string, markers []string) bool {
	for _, m := range markers {
		if strings.Contains(text, m) {
			return true
		}
	}
	return false
}

func classify(res proc.Result, extra map[error][]string) error {
	if res.Success() {
		return nil
	}
	stderr := string(res.Stderr)
	lower := strings.ToLower(stderr)
	if containsAny(lower, authMarkers) {
		return errors.New(strings.TrimSpace(stderr))
	}
	if containsAny(lower, offlineMarkers) {
		return ErrOffline
	}
	for err, markers := range extra {
		if containsAny(stderr, markers) {
			return err
		}
	}
	return errors.New(strings.TrimSpace(stderr))
}

func runClassified(
	runner func(dir string, args ...string) (proc.Result, error),
	dir string, extra map[error][]string, args ...string,
) error {
	res, err := runner(dir, args...)
	if err != nil {
		return err
	}
	return classify(res, extra)
}

func runNet(dir string, extra map[error][]string, args ...string) error {
	return runClassified(gitNet, dir, extra, args...)
}

func runLocal(dir string, args ...string) error {
	return runClassified(git, dir, nil, args...)
}

func HasUnsyncedChanges(dir, remote, branch string) bool {
	status, err := git(dir, "status", "--porcelain", "--", itemsFile)
	if err == nil && len(status.Stdout) > 0 {
		return true
	}
	ahead, err := git(dir, "rev-list", "--count", remote+"/"+branch+"..HEAD")
	if err != nil || !ahead.Success() {
		return false
	}
	count, err := strconv.Atoi(strings.TrimSpace(string(ahead.Stdout)))
	return err == nil && count > 0
}

func PullRebase(dir, remote, branch string) error {
	conflict := map[error][]string{ErrConflict: conflictMarkers}
	return runNet(dir, conflict, "pull", "--rebase", "--autostash", remote, branch)
}

func Push(dir, remote, branch string) error {
	return runNet(dir, map[error][]string{ErrRejected: rejectedMarkers}, "push", remote, branch)
}

func StageAndCommit(dir, message string) error {
	if _, err := git(dir, "add", itemsFile); err != nil {
		return err
	}
	res, err := git(dir, "commit", "-m", message)
	if err != nil || res.Success() {
		return err
	}
	if strings.Contains(string(res.Stdout)+string(res.Stderr), "nothing to commit") {
		return nil
	}
	return errors.New(strings.TrimSpace(string(res.Stderr)))
}

func ResolveConflictKeepRemote(dir, remote, branch string) (string, error) {
	_, _ = git(dir, "rebase", "--abort")
	backup := filepath.Join(dir, fmt.Sprintf("%s.local-backup-%d", itemsFile, time.Now().Unix()))
	_ = copyFile(filepath.Join(dir, itemsFile), backup)
	if err := runNet(dir, nil, "fetch", remote, branch); err != nil {
		return "", err
	}
	saveMergeBase(dir, remote, branch, BasePathFor(backup))
	if err := runLocal(dir, "reset", "--hard", remote+"/"+branch); err != nil {
		return "", err
	}
	return backup, nil
}

func BasePathFor(backup string) string { return backup + ".base" }

func saveMergeBase(dir, remote, branch, destination string) {
	res, err := git(dir, "merge-base", "HEAD", remote+"/"+branch)
	if err != nil || !res.Success() {
		return
	}
	base := strings.TrimSpace(string(res.Stdout))
	shown, err := git(dir, "show", base+":"+itemsFile)
	if err != nil || !shown.Success() {
		return
	}
	_ = os.WriteFile(destination, shown.Stdout, 0o644)
}

func copyFile(from, to string) error {
	src, err := os.Open(from)
	if err != nil {
		return err
	}
	defer src.Close()
	dst, err := os.Create(to)
	if err != nil {
		return err
	}
	defer dst.Close()
	_, err = io.Copy(dst, src)
	return err
}

const (
	lockName    = "hopes-sync.lock"
	lockStale   = 2 * time.Minute
	lockTimeout = 20 * time.Second
)

func Lock(dir string) (func(), error) {
	path := filepath.Join(dir, ".git", lockName)
	deadline := time.Now().Add(lockTimeout)
	for {
		f, err := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0o644)
		if err == nil {
			_, _ = fmt.Fprintf(f, "%d\n", os.Getpid())
			_ = f.Close()
			return func() { _ = os.Remove(path) }, nil
		}
		if !errors.Is(err, os.ErrExist) {
			return nil, err
		}
		if info, statErr := os.Stat(path); statErr == nil && time.Since(info.ModTime()) > lockStale {
			_ = os.Remove(path)
			continue
		}
		if time.Now().After(deadline) {
			return nil, ErrLocked
		}
		time.Sleep(100 * time.Millisecond)
	}
}
