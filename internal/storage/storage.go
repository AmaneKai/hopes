package storage

import (
	"cmp"
	"encoding/json/jsontext"
	"encoding/json/v2"
	"errors"
	"fmt"
	"strings"

	"github.com/AmaneKai/hopes/internal/config"
	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/paths"
)

type Storage interface {
	Load() ([]domain.Item, error)
	Save(items []domain.Item) error
	DataDir() string
}

func Open(cfg config.Storage, fallbackDir string) (Storage, error) {
	dir := cmp.Or(cfg.Dir, fallbackDir, paths.Data())
	switch cfg.Backend {
	case config.BackendExec:
		return NewExecStore(dir, cfg.Command)
	case config.BackendJSON:
		return NewJSONStore(dir), nil
	default:
		return nil, fmt.Errorf("unknown storage backend %q", cfg.Backend)
	}
}

func decodeItems(text string) ([]domain.Item, error) {
	if strings.TrimSpace(text) == "" {
		return []domain.Item{}, nil
	}
	var items []domain.Item
	if err := json.Unmarshal([]byte(text), &items); err != nil {
		return nil, err
	}
	if items == nil {
		return nil, errors.New("tasks file holds null instead of a list")
	}
	domain.AssignMissingIDs(items)
	return items, nil
}

func encodeItems(items []domain.Item, indent string) ([]byte, error) {
	if indent != "" {
		return json.Marshal(items, jsontext.WithIndent(indent))
	}
	return json.Marshal(items)
}
