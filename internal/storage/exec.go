package storage

import (
	"errors"
	"fmt"
	"slices"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/proc"
	"github.com/AmaneKai/hopes/internal/textfile"
)

const (
	execOutputLimit = 64 << 20
	execErrorLimit  = 160
)

type ExecStore struct {
	dir     string
	program string
	args    []string
}

func NewExecStore(dir string, command []string) (*ExecStore, error) {
	if len(command) == 0 || strings.TrimSpace(command[0]) == "" {
		return nil, errors.New("storage.command is required for the exec backend")
	}
	return &ExecStore{
		dir:     dir,
		program: command[0],
		args:    slices.Clone(command[1:]),
	}, nil
}

func (s *ExecStore) DataDir() string { return s.dir }

func (s *ExecStore) run(verb string, input []byte) (proc.Result, error) {
	args := append(slices.Clone(s.args), verb)
	res, err := proc.Run(s.program, args, "", input, execOutputLimit, 0)
	if err != nil {
		return res, err
	}
	if !res.Success() {
		return res, commandFailed(verb, res)
	}
	return res, nil
}

func commandFailed(verb string, res proc.Result) error {
	if res.TimedOut {
		return fmt.Errorf("storage command `%s` timed out", verb)
	}
	detail := []rune(strings.TrimSpace(string(res.Stderr)))
	detail = detail[:min(len(detail), execErrorLimit)]
	return fmt.Errorf("storage command `%s` failed (exit status %d): %s",
		verb, res.ExitCode, string(detail))
}

func (s *ExecStore) Load() ([]domain.Item, error) {
	res, err := s.run("load", nil)
	if err != nil {
		return nil, err
	}
	return decodeItems(textfile.Normalize(res.Stdout))
}

func (s *ExecStore) Save(items []domain.Item) error {
	payload, err := encodeItems(items, "")
	if err != nil {
		return err
	}
	_, err = s.run("save", payload)
	return err
}
