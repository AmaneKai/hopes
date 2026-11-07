package viewstate

import (
	"encoding/json"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

func state() *domain.AppState {
	items := []domain.Item{domain.NewItem("a", "Inbox"), domain.NewItem("b", "Uni")}
	return domain.NewAppState(items, gitsync.Status{Kind: gitsync.Disabled})
}

func TestTheLastWorkspaceAndViewAreRestored(t *testing.T) {
	saved := state()
	saved.Workspace, saved.ViewMode, saved.SortMode = "Uni", domain.Kanban, domain.SortPriority
	data, _ := json.Marshal(Capture(saved))
	var v ViewState
	if err := json.Unmarshal(data, &v); err != nil {
		t.Fatal(err)
	}
	fresh := state()
	v.Apply(fresh)
	ok := fresh.Workspace == "Uni" && fresh.ViewMode == domain.Kanban &&
		fresh.SortMode == domain.SortPriority
	if !ok || len(fresh.FilteredIndices()) != 1 {
		t.Fatalf("not restored: %+v", fresh)
	}
}

func TestAWorkspaceThatHasEmptiedIsNotReopened(t *testing.T) {
	fresh := state()
	ViewState{Workspace: "Gone"}.Apply(fresh)
	if fresh.Workspace != "Inbox" {
		t.Fatalf("got %q", fresh.Workspace)
	}
}

func TestAViewFileFromTheRustBuildStillLoads(t *testing.T) {
	old := `{"view_mode":"Table","sort_mode":"Natural","nav_section":"Todo","tag_filter":"#uni"}`
	var v ViewState
	if err := json.Unmarshal([]byte(old), &v); err != nil || v.Workspace != "" {
		t.Fatalf("got %+v %v", v, err)
	}
}
