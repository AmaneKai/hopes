package viewstate

import (
	"encoding/json/v2"
	"os"
	"path/filepath"
	"slices"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/paths"
)

type ViewState struct {
	ViewMode  domain.ViewMode `json:"view_mode"`
	SortMode  domain.SortMode `json:"sort_mode"`
	Workspace string          `json:"workspace,omitzero"`
}

func path() string { return filepath.Join(paths.State(), "view.json") }

func Capture(s *domain.AppState) ViewState {
	if s == nil {
		return ViewState{}
	}
	return ViewState{ViewMode: s.ViewMode, SortMode: s.SortMode, Workspace: s.Workspace}
}

func (v ViewState) Apply(s *domain.AppState) {
	if s == nil {
		return
	}
	s.ViewMode = v.ViewMode
	s.SortMode = v.SortMode
	if name, ok := s.FindWorkspace(v.Workspace); ok && hasTasks(s, name) {
		s.Workspace = name
	}
	s.RefreshCache()
}

func hasTasks(s *domain.AppState, name string) bool {
	return slices.ContainsFunc(s.Workspaces(), func(w domain.WorkspaceSummary) bool {
		return w.Name == name && w.Total > 0
	})
}

func Load() (ViewState, bool) {
	data, err := os.ReadFile(path())
	if err != nil {
		return ViewState{}, false
	}
	var v ViewState
	return v, json.Unmarshal(data, &v) == nil
}

func (v ViewState) Save() {
	if err := os.MkdirAll(paths.State(), 0o755); err != nil {
		return
	}
	if data, err := json.Marshal(v); err == nil {
		_ = os.WriteFile(path(), data, 0o644)
	}
}
