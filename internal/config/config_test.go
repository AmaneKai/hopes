package config

import (
	"os"
	"path/filepath"
	"slices"
	"testing"
	"time"

	"github.com/AmaneKai/hopes/internal/paths"
	"github.com/AmaneKai/hopes/internal/textfile"
)

func TestDefaultsWhenEmpty(t *testing.T) {
	c, err := Parse("")
	defaults := c.Sync.Debounce == 5*time.Second && c.Sync.Enabled &&
		c.Storage.Backend == BackendJSON
	if err != nil || !defaults {
		t.Fatalf("unexpected defaults %+v %v", c, err)
	}
}

func TestOverrides(t *testing.T) {
	c, err := Parse(`
[sync]
enabled = false   # comment
debounce_seconds = 9
data_dir = "/x"
remote = 'up'
branch = "dev"
pull_on_startup = false
unknown = 1

[storage]
backend = "exec"
command = [
  "a", "b#c",
]
dir = "/d"

[other]
x = 1
`)
	if err != nil {
		t.Fatal(err)
	}
	ok := !c.Sync.Enabled && c.Sync.Debounce == 9*time.Second && c.Sync.DataDir == "/x" &&
		c.Sync.Remote == "up" && c.Sync.Branch == "dev" && !c.Sync.PullOnStartup &&
		c.Storage.Backend == BackendExec && c.Storage.Dir == "/d" &&
		slices.Equal(c.Storage.Command, []string{"a", "b#c"})
	if !ok {
		t.Fatalf("overrides not applied: %+v", c)
	}
}

func TestInvalidConfigsFallBackToDefaults(t *testing.T) {
	bad := []string{
		"[sync]\ndebounce_seconds = -1\n",
		"[sync]\nenabled = \"yes\"\n",
		"[storage]\nbackend = \"mongo\"\n",
		"[storage]\ncommand = [1]\n",
		"sync = 3\n",
		"not toml at all [",
		"[sync]\nenabled = true\nenabled = false\n",
	}
	for _, text := range bad {
		if _, err := Parse(text); err == nil {
			t.Errorf("expected an error for %q", text)
		}
	}
}

func TestToleratesBOMAndCRLF(t *testing.T) {
	c, err := Parse(textfile.Normalize([]byte("\xEF\xBB\xBF[sync]\r\nenabled = false\r\n")))
	if err != nil || c.Sync.Enabled {
		t.Fatalf("got %+v %v", c, err)
	}
}

func TestLoadSurfacesBrokenConfigInsteadOfSilentDefaults(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", dir)
	t.Setenv("HOME", dir)
	t.Setenv("AppData", dir)
	location := filepath.Join(paths.Config(), "config.toml")
	if err := os.MkdirAll(filepath.Dir(location), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(location, []byte("[sync\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := Load(); err == nil {
		t.Fatal("expected a parse error for a broken config file")
	}
}

func TestUnknownKeysProduceWarningsNotErrors(t *testing.T) {
	c, err := Parse("typo = 1\n[sync]\ndebounce_secs = 3\n[other]\nx = 1\n")
	want := []string{"unknown key other.x", "unknown key sync.debounce_secs", "unknown key typo"}
	if err != nil || !slices.Equal(c.Warnings, want) {
		t.Fatalf("got %v, %v", c.Warnings, err)
	}
}
