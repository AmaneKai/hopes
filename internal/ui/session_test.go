package ui

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
	"github.com/AmaneKai/hopes/internal/storage"
)

type session struct {
	t   *testing.T
	app *App
	dir string
}

func seed() []domain.Item {
	items := []domain.Item{
		domain.NewItem("Write compiler backend", "Work"),
		domain.NewItem("Design responsive TUI", "Work"),
		domain.NewItem("Implement undo buffer", "Work"),
	}
	items[0].Priority, items[1].Priority = domain.Urgent, domain.High
	return items
}

func twoWorkspaces() []domain.Item {
	return []domain.Item{
		domain.NewItem("w1", "Work"), domain.NewItem("h1", "Home"),
		domain.NewItem("w2", "Work"), domain.NewItem("h2", "Home"),
	}
}

func idleSync(dir string) *gitsync.Engine {
	return gitsync.Spawn(gitsync.Options{Dir: dir, Remote: "origin", Branch: "main"})
}

func openApp(dir string) *App { return New(storage.NewJSONStore(dir), idleSync(dir)) }

func newSession(t *testing.T, items []domain.Item) *session {
	dir := t.TempDir()
	must(t, storage.NewJSONStore(dir).Save(items))
	return &session{t: t, app: openApp(dir), dir: dir}
}

func must(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func writeFile(t *testing.T, dir, name, content string) string {
	t.Helper()
	path := filepath.Join(dir, name)
	must(t, os.WriteFile(path, []byte(content), 0o644))
	return path
}

func (s *session) press(keys ...Key) {
	for _, k := range keys {
		s.app.HandleKey(k)
	}
}

func (s *session) typeStr(text string) {
	for _, r := range text {
		s.press(Rune(r))
	}
}

func (s *session) command(line string) {
	s.press(Rune(':'))
	s.typeStr(line)
	s.press(Special(KeyEnter))
}

func (s *session) items() []domain.Item { return s.app.State.Items }

func (s *session) cursor() int {
	if idx, ok := s.app.SelectedIndex(); ok {
		return idx
	}
	return -1
}

func (s *session) saved() []domain.Item {
	items, err := storage.NewJSONStore(s.dir).Load()
	must(s.t, err)
	return items
}

func (s *session) screen(w, h int) string { return s.app.Render(w, h).PlainText() }

func (s *session) status() string { return s.app.State.Status() }

func expect[T comparable](t *testing.T, got, want T) {
	t.Helper()
	if got != want {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func expectTrue(t *testing.T, ok bool, context string) {
	t.Helper()
	if !ok {
		t.Fatal(context)
	}
}

func expectContains(t *testing.T, text, part string) {
	t.Helper()
	expectTrue(t, strings.Contains(text, part), "missing "+part+" in:\n"+text)
}

func titles(items []domain.Item) string {
	names := make([]string, len(items))
	for i, it := range items {
		names[i] = it.Title
	}
	return strings.Join(names, ",")
}

func expectVisible(t *testing.T, s *session, want ...int) {
	t.Helper()
	got := s.app.State.FilteredIndices()
	expectTrue(t, slices.Equal(got, want), fmt.Sprintf("visible %v, want %v", got, want))
}

func hasWorkspaceNamed(s *session, name string) bool {
	_, ok := s.app.State.FindWorkspace(name)
	return ok
}

var syncPulled = gitsync.Status{Kind: gitsync.UpdatedRemoteData}
