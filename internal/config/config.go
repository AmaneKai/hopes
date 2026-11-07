package config

import (
	"errors"
	"os"
	"path/filepath"
	"time"

	"github.com/AmaneKai/hopes/internal/paths"
	"github.com/AmaneKai/hopes/internal/textfile"
)

type Backend string

const (
	BackendJSON Backend = "json"
	BackendExec Backend = "exec"
)

type Sync struct {
	Enabled       bool
	Debounce      time.Duration
	DataDir       string
	Remote        string
	Branch        string
	PullOnStartup bool
}

type Storage struct {
	Backend Backend
	Dir     string
	Command []string
}

type Config struct {
	Sync    Sync
	Storage Storage
}

var errType = errors.New("config value has the wrong type")

func Default() Config {
	return Config{
		Sync: Sync{
			Enabled:       true,
			Debounce:      5 * time.Second,
			Remote:        "origin",
			Branch:        "main",
			PullOnStartup: true,
		},
		Storage: Storage{Backend: BackendJSON},
	}
}

func Load() Config {
	data, err := os.ReadFile(filepath.Join(paths.Config(), "config.toml"))
	if err != nil {
		return Default()
	}
	cfg, err := Parse(textfile.Normalize(data))
	if err != nil {
		return Default()
	}
	return cfg
}

func Parse(text string) (Config, error) {
	doc, err := parseTOML(text)
	if err != nil {
		return Default(), err
	}
	if sectionShadowed(doc, "sync", "storage") {
		return Default(), errType
	}
	cfg := Default()
	sync, store := doc["sync"], doc["storage"]
	seconds := int64(cfg.Sync.Debounce / time.Second)
	err = errors.Join(
		read(sync, "enabled", &cfg.Sync.Enabled),
		read(sync, "debounce_seconds", &seconds),
		read(sync, "remote", &cfg.Sync.Remote),
		read(sync, "branch", &cfg.Sync.Branch),
		read(sync, "pull_on_startup", &cfg.Sync.PullOnStartup),
		read(sync, "data_dir", &cfg.Sync.DataDir),
		read(store, "dir", &cfg.Storage.Dir),
		readStrings(store, "command", &cfg.Storage.Command),
		readBackend(store, &cfg.Storage.Backend),
	)
	if err != nil || seconds < 0 {
		return Default(), errors.Join(err, errType)
	}
	cfg.Sync.Debounce = time.Duration(seconds) * time.Second
	return cfg, nil
}

func read[T any](t table, key string, slot *T) error {
	raw, ok := t[key]
	if !ok {
		return nil
	}
	value, ok := raw.(T)
	if !ok {
		return errType
	}
	*slot = value
	return nil
}

func readStrings(t table, key string, slot *[]string) error {
	var raw []any
	if err := read(t, key, &raw); err != nil || raw == nil {
		return err
	}
	out := make([]string, 0, len(raw))
	for _, v := range raw {
		s, ok := v.(string)
		if !ok {
			return errType
		}
		out = append(out, s)
	}
	*slot = out
	return nil
}

func readBackend(t table, slot *Backend) error {
	var name string
	if err := read(t, "backend", &name); err != nil || name == "" {
		return err
	}
	backend := Backend(name)
	if backend != BackendJSON && backend != BackendExec {
		return errType
	}
	*slot = backend
	return nil
}

func sectionShadowed(doc map[string]table, names ...string) bool {
	for _, name := range names {
		if _, ok := doc[""][name]; ok {
			return true
		}
	}
	return false
}
