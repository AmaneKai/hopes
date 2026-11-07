package storage

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
)

func TestJSONStoreRoundTripsAndWritesAtomically(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "one", "two", "three")
	store := NewJSONStore(dir)
	items := []domain.Item{domain.NewItem("a <b> & c", "Work")}
	if err := store.Save(items); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(filepath.Join(dir, "items.tmp")); !os.IsNotExist(err) {
		t.Fatal("temporary file left behind")
	}
	loaded, err := store.Load()
	if err != nil || len(loaded) != 1 || loaded[0] != items[0] {
		t.Fatalf("got %+v %v", loaded, err)
	}
}

func TestJSONStoreWritesAnEmptyListNotNull(t *testing.T) {
	dir := t.TempDir()
	if err := NewJSONStore(dir).Save(nil); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "items.json"))
	if string(data) != "[]" {
		t.Fatalf("got %q", data)
	}
}

func TestJSONStoreReadsFilesSavedByOtherEditors(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "items.json")
	files := []string{
		"\xFF\xFE[\x00]\x00\r\x00\n\x00",
		"\xEF\xBB\xBF[{\"title\":\"t\",\"priority\":\"Low\",\"status\":\"Todo\"}]",
	}
	for want, content := range files {
		os.WriteFile(path, []byte(content), 0o644)
		items, err := NewJSONStore(dir).Load()
		if err != nil || len(items) != want {
			t.Fatalf("file %d: got %d items, %v", want, len(items), err)
		}
	}
}

func TestMissingFieldsAreRejected(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "items.json"), []byte(`[{"title":"t","status":"Todo"}]`), 0o644)
	if _, err := NewJSONStore(dir).Load(); err == nil {
		t.Fatal("a task without a priority should not load")
	}
}

func TestExecStoreRoundTripsThroughAHelper(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("uses sh")
	}
	dir := t.TempDir()
	db := filepath.Join(dir, "db.json")
	script := `case "$1" in load) cat "$0" 2>/dev/null ;; save) cat > "$0" ;; esac`
	store, err := NewExecStore(dir, []string{"sh", "-c", script, db})
	if err != nil {
		t.Fatal(err)
	}
	items := []domain.Item{domain.NewItem("a", "x")}
	if err := store.Save(items); err != nil {
		t.Fatal(err)
	}
	loaded, err := store.Load()
	if err != nil || len(loaded) != 1 || loaded[0] != items[0] {
		t.Fatalf("got %+v %v", loaded, err)
	}
}

func TestExecStoreNeedsACommand(t *testing.T) {
	if _, err := NewExecStore(t.TempDir(), nil); err == nil {
		t.Fatal("expected an error")
	}
}
