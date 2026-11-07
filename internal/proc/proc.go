package proc

import (
	"bytes"
	"context"
	"errors"
	"os/exec"
	"time"
)

const pipeGrace = time.Second

type Result struct {
	ExitCode int
	TimedOut bool
	Stdout   []byte
	Stderr   []byte
}

func (r Result) Success() bool { return !r.TimedOut && r.ExitCode == 0 }

type capped struct {
	buf   bytes.Buffer
	limit int
}

func (c *capped) Write(p []byte) (int, error) {
	if room := c.limit - c.buf.Len(); room > 0 {
		c.buf.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}

func Run(
	name string, args []string, dir string, input []byte, limit int, timeout time.Duration,
) (Result, error) {
	ctx := context.Background()
	if timeout > 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, timeout)
		defer cancel()
	}
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir
	cmd.WaitDelay = pipeGrace
	if input != nil {
		cmd.Stdin = bytes.NewReader(input)
	}
	stdout, stderr := &capped{limit: limit}, &capped{limit: limit}
	cmd.Stdout, cmd.Stderr = stdout, stderr

	if err := cmd.Start(); err != nil {
		return Result{}, err
	}
	err := cmd.Wait()
	res := Result{Stdout: stdout.buf.Bytes(), Stderr: stderr.buf.Bytes()}
	if ctx.Err() == context.DeadlineExceeded {
		res.TimedOut, res.ExitCode = true, -1
		return res, nil
	}
	var exitErr *exec.ExitError
	switch {
	case err == nil:
	case errors.As(err, &exitErr):
		res.ExitCode = exitErr.ExitCode()
	case errors.Is(err, exec.ErrWaitDelay):
	default:
		return res, err
	}
	return res, nil
}
