package gitsync

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/AmaneKai/hopes/internal/proc"
)

func TestLockIsExclusiveAndReleasable(t *testing.T) {
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, ".git"), 0o755); err != nil {
		t.Fatal(err)
	}
	unlock, err := Lock(dir)
	if err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() {
		u, err := Lock(dir)
		if err == nil {
			u()
		}
		done <- err
	}()
	select {
	case <-done:
		t.Fatal("second Lock acquired while first was held")
	case <-time.After(300 * time.Millisecond):
	}
	unlock()
	if err := <-done; err != nil {
		t.Fatalf("second Lock after release: %v", err)
	}
}

func TestClassifyDistinguishesAuthFromOffline(t *testing.T) {
	cases := []struct {
		stderr  string
		offline bool
	}{
		{"fatal: unable to access 'x': Could not resolve host: github.com", true},
		{"fatal: unable to access 'x': The requested URL returned error: 403", false},
		{"git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.", false},
		{"fatal: Authentication failed for 'x'", false},
	}
	for _, c := range cases {
		res := resultWithStderr(c.stderr)
		err := classify(res, nil)
		if errors.Is(err, ErrOffline) != c.offline {
			t.Errorf("%q: offline=%v, want %v", c.stderr, errors.Is(err, ErrOffline), c.offline)
		}
	}
}

func resultWithStderr(s string) proc.Result {
	return proc.Result{ExitCode: 1, Stderr: []byte(s)}
}
