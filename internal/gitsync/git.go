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
	offlineMarkers = []string{
		"could not resolve host",
		"unable to access",
		"connection timed out",
		"network is unreachable",
		"could not read from remote repository",
		"no route to host",
	}
	rejectedMarkers = []string{"[rejected]", "non-fast-forward", "fetch first"}
	conflictMarkers = []string{"CONFLICT", "could not apply"}
)

func IsRepo(dir string) bool {
	_, err := os.Stat(filepath.Join(dir, ".git"))
	return err == nil
}

func git(dir string, args ...string) (proc.Result, error) {
	res, err := proc.Run("git", args, dir, nil, outputLimit, networkTimeout)
	if err != nil {
		return res, err
	}
	if res.TimedOut {
		return res, ErrOffline
	}
	return res, nil
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
	if containsAny(strings.ToLower(stderr), offlineMarkers) {
		return ErrOffline
	}
	for err, markers := range extra {
		if containsAny(stderr, markers) {
			return err
		}
	}
	return errors.New(strings.TrimSpace(stderr))
}

func run(dir string, extra map[error][]string, args ...string) error {
	res, err := git(dir, args...)
	if err != nil {
		return err
	}
	return classify(res, extra)
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
	return run(dir, conflict, "pull", "--rebase", "--autostash", remote, branch)
}

func Push(dir, remote, branch string) error {
	return run(dir, map[error][]string{ErrRejected: rejectedMarkers}, "push", remote, branch)
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
	if _, err := git(dir, "fetch", remote, branch); err != nil {
		return "", err
	}
	if _, err := git(dir, "reset", "--hard", remote+"/"+branch); err != nil {
		return "", err
	}
	return backup, nil
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
