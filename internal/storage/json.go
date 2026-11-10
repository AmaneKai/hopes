package storage

import (
	"bytes"
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"sync"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/textfile"
)

var ErrChangedOnDisk = errors.New("tasks changed on disk since they were loaded")

type JSONStore struct {
	path string

	mu    sync.Mutex
	known []byte
	seen  bool
}

func NewJSONStore(dir string) *JSONStore {
	return &JSONStore{path: filepath.Join(dir, "items.json")}
}

func (s *JSONStore) DataDir() string { return filepath.Dir(s.path) }

func (s *JSONStore) Load() ([]domain.Item, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	data, err := os.ReadFile(s.path)
	if errors.Is(err, fs.ErrNotExist) {
		s.known, s.seen = nil, true
		return []domain.Item{}, nil
	}
	if err != nil {
		return nil, err
	}
	s.known, s.seen = data, true
	return decodeItems(textfile.Normalize(data))
}

func (s *JSONStore) Save(items []domain.Item) error {
	data, err := encodeItems(items, "  ")
	if err != nil {
		return err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.seen {
		current, err := os.ReadFile(s.path)
		if err != nil && !errors.Is(err, fs.ErrNotExist) {
			return err
		}
		if !bytes.Equal(current, s.known) {
			return ErrChangedOnDisk
		}
	}
	if err := textfile.WriteAtomic(s.path, data); err != nil {
		return err
	}
	s.known, s.seen = data, true
	return nil
}
