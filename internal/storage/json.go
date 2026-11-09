package storage

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/textfile"
)

type JSONStore struct {
	path string
}

func NewJSONStore(dir string) *JSONStore {
	_ = os.MkdirAll(dir, 0o755)
	return &JSONStore{path: filepath.Join(dir, "items.json")}
}

func (s *JSONStore) DataDir() string { return filepath.Dir(s.path) }

func (s *JSONStore) Load() ([]domain.Item, error) {
	data, err := os.ReadFile(s.path)
	if errors.Is(err, fs.ErrNotExist) {
		return []domain.Item{}, nil
	}
	if err != nil {
		return nil, err
	}
	return decodeItems(textfile.Normalize(data))
}

func (s *JSONStore) Save(items []domain.Item) error {
	data, err := encodeItems(items, "  ")
	if err != nil {
		return err
	}
	dir := filepath.Dir(s.path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	tmp := filepath.Join(dir, "items.tmp")
	if err := writeSynced(tmp, data); err != nil {
		_ = os.Remove(tmp)
		return err
	}
	if err := os.Rename(tmp, s.path); err != nil {
		_ = os.Remove(tmp)
		return err
	}
	return nil
}

func writeSynced(path string, data []byte) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	if _, err := f.Write(data); err != nil {
		_ = f.Close()
		return err
	}
	if err := f.Sync(); err != nil {
		_ = f.Close()
		return err
	}
	return f.Close()
}
