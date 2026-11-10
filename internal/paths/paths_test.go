package paths

import (
	"path/filepath"
	"testing"
)

func TestXDGOverrideMustBeAbsolute(t *testing.T) {
	t.Setenv("TEST_XDG", "relative/dir")
	if got, want := xdg("TEST_XDG", "fallback"), filepath.Join(home(), "fallback"); got != want {
		t.Fatalf("got %q, want %q", got, want)
	}
	absolute := t.TempDir()
	t.Setenv("TEST_XDG", absolute)
	if got := xdg("TEST_XDG", "fallback"); got != absolute {
		t.Fatalf("got %q, want %q", got, absolute)
	}
}
