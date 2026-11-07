//go:build !windows

package proc

import (
	"bytes"
	"strings"
	"testing"
	"time"
)

func sh(script string, input []byte, limit int, timeout time.Duration) (Result, error) {
	return Run("sh", []string{"-c", script, "hopes"}, "", input, limit, timeout)
}

func TestArgumentsSurviveUntouched(t *testing.T) {
	args := []string{"[%s]\n", "with space", "quote\"inside", "trailing\\", "", "雨音 カイ"}
	r, err := Run("printf", args, "", nil, 1<<16, 0)
	want := "[with space]\n[quote\"inside]\n[trailing\\]\n[]\n[雨音 カイ]\n"
	if err != nil || string(r.Stdout) != want {
		t.Fatalf("got %q %v", r.Stdout, err)
	}
}

func TestLargePayloadRoundTripsWithoutDeadlock(t *testing.T) {
	data := []byte(strings.Repeat("0123456789abcdef", 200_000))
	r, err := Run("cat", nil, "", data, 1<<24, 20*time.Second)
	if err != nil || !r.Success() || !bytes.Equal(r.Stdout, data) {
		t.Fatalf("round trip failed: %v", err)
	}
}

func TestChattyChildCannotStallOnAFullPipe(t *testing.T) {
	script := "head -c 3000000 /dev/zero; head -c 3000000 /dev/zero >&2"
	r, err := sh(script, nil, 1000, 20*time.Second)
	if err != nil || !r.Success() || len(r.Stdout) != 1000 || len(r.Stderr) != 1000 {
		t.Fatalf("got %d/%d bytes, %v", len(r.Stdout), len(r.Stderr), err)
	}
}

func TestHungChildIsKilledAtTheDeadline(t *testing.T) {
	started := time.Now()
	r, err := sh("sleep 30", nil, 1000, 400*time.Millisecond)
	if err != nil || !r.TimedOut || time.Since(started) > 10*time.Second {
		t.Fatalf("not killed in time: %+v %v", r, err)
	}
}

func TestFailureReportsStatusAndStderr(t *testing.T) {
	r, err := sh("echo boom from helper >&2; exit 3", nil, 1000, 0)
	if err != nil || r.ExitCode != 3 || !strings.Contains(string(r.Stderr), "boom from helper") {
		t.Fatalf("got %+v %v", r, err)
	}
}

func TestMissingProgramIsAnError(t *testing.T) {
	if _, err := Run("hopes-no-such-program-xyz", nil, "", nil, 1000, 0); err == nil {
		t.Fatal("expected an error")
	}
}
