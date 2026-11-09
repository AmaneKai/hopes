package domain

import (
	"cmp"
	"fmt"
	"slices"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/AmaneKai/hopes/internal/gitsync"
)

func Snippet(text string, maxLen int) string {
	if maxLen <= 0 {
		return ""
	}
	if utf8.RuneCountInString(text) <= maxLen {
		return text
	}
	return string([]rune(text)[:maxLen]) + "..."
}

func enumAt[T ~int](v T, values []string) string {
	if int(v) < 0 || int(v) >= len(values) {
		return values[0]
	}
	return values[v]
}

func cycleEnum[T ~int](v T, count, delta int) T {
	n := max(1, count)
	return T(((int(v)+delta)%n + n) % n)
}

func marshalEnum[T ~int](v T, names []string) ([]byte, error) {
	if int(v) < 0 || int(v) >= len(names) {
		return nil, fmt.Errorf("invalid value %d", v)
	}
	return []byte(names[v]), nil
}

func unmarshalEnum[T ~int](b []byte, names []string, out *T) error {
	if i := slices.Index(names, string(b)); i >= 0 {
		*out = T(i)
		return nil
	}
	return fmt.Errorf("unknown value %q", b)
}

type ViewMode int

const (
	Table ViewMode = iota
	Kanban
)

var viewModeNames = [...]string{"Table", "Kanban"}

func (v ViewMode) MarshalText() ([]byte, error)  { return marshalEnum(v, viewModeNames[:]) }
func (v *ViewMode) UnmarshalText(b []byte) error { return unmarshalEnum(b, viewModeNames[:], v) }

type FocusPane int

const (
	FocusMain FocusPane = iota
	FocusNav
)

func (f FocusPane) Other() FocusPane {
	if f == FocusMain {
		return FocusNav
	}
	return FocusMain
}

type SortMode int

const (
	SortNatural SortMode = iota
	SortPriority
	SortStatus
	SortTitle
)

var sortModeNames = [...]string{"Natural", "Priority", "Status", "Title"}

func (s SortMode) Next() SortMode                { return cycleEnum(s, len(sortModeNames), 1) }
func (s SortMode) Label() string                 { return enumAt(s, sortModeNames[:]) }
func (s SortMode) MarshalText() ([]byte, error)  { return marshalEnum(s, sortModeNames[:]) }
func (s *SortMode) UnmarshalText(b []byte) error { return unmarshalEnum(b, sortModeNames[:], s) }

type WorkspaceSummary struct {
	Name  string
	Open  int
	Total int
}

type WorkspaceStats struct {
	Total, Todo, InProgress, Complete, Percent int
}

const StatusTTL = 4 * time.Second

type Deleted struct {
	Index int
	Item  Item
}

type AppState struct {
	Items       []Item
	ViewMode    ViewMode
	Focus       FocusPane
	SortMode    SortMode
	Workspace   string
	Filter      string
	HasFilter   bool
	LastDeleted *Deleted
	Selection   int
	SyncStatus  gitsync.Status

	statusMessage string
	statusAt      time.Time

	filtered   []int
	stats      WorkspaceStats
	workspaces []WorkspaceSummary
	columns    [3][]int
}

func NewAppState(items []Item, sync gitsync.Status) *AppState {
	workspace := DefaultWorkspace
	if len(items) > 0 {
		workspace = cmp.Or(items[0].Workspace, DefaultWorkspace)
	}
	s := &AppState{Items: items, Workspace: workspace, Selection: -1, SyncStatus: sync}
	s.RefreshCache()
	return s
}

func (s *AppState) FilteredIndices() []int         { return s.filtered }
func (s *AppState) KanbanColumn(st Status) []int   { return s.columns[st.Column()] }
func (s *AppState) Workspaces() []WorkspaceSummary { return s.workspaces }
func (s *AppState) Stats() WorkspaceStats          { return s.stats }

func (s *AppState) SelectedItem() *Item {
	if s.Selection < 0 || s.Selection >= len(s.Items) {
		return nil
	}
	return &s.Items[s.Selection]
}

func (s *AppState) WorkspacePosition() int {
	return max(0, indexByName(s.workspaces, s.Workspace))
}

func (s *AppState) WorkspaceAtOffset(forward bool) (string, bool) {
	if len(s.workspaces) < 2 {
		return "", false
	}
	delta := -1
	if forward {
		delta = 1
	}
	next := cycleEnum(s.WorkspacePosition(), len(s.workspaces), delta)
	return s.workspaces[next].Name, true
}

func (s *AppState) FindWorkspace(name string) (string, bool) {
	i := slices.IndexFunc(s.workspaces, func(w WorkspaceSummary) bool {
		return strings.EqualFold(w.Name, name)
	})
	if i < 0 {
		return "", false
	}
	return s.workspaces[i].Name, true
}

func (s *AppState) SetStatus(msg string) {
	s.statusMessage = msg
	s.statusAt = time.Now()
}

func (s *AppState) Status() string {
	if s.statusMessage == "" || time.Since(s.statusAt) >= StatusTTL {
		return ""
	}
	return s.statusMessage
}

func (s *AppState) RefreshCache() {
	s.refreshWorkspaces()
	s.refreshVisible()
}

func (s *AppState) refreshWorkspaces() {
	var workspaces []WorkspaceSummary
	var stats WorkspaceStats
	for _, item := range s.Items {
		i := indexByName(workspaces, item.Workspace)
		if i < 0 {
			i = len(workspaces)
			workspaces = append(workspaces, WorkspaceSummary{Name: item.Workspace})
		}
		workspaces[i].Total++
		if item.Status != Complete {
			workspaces[i].Open++
		}
		if item.Workspace == s.Workspace {
			stats.Total++
			switch item.Status {
			case Todo:
				stats.Todo++
			case InProgress:
				stats.InProgress++
			case Complete:
				stats.Complete++
			}
		}
	}
	if indexByName(workspaces, s.Workspace) < 0 {
		workspaces = append(workspaces, WorkspaceSummary{Name: s.Workspace})
	}
	if stats.Total > 0 {
		stats.Percent = stats.Complete * 100 / stats.Total
	}
	s.stats = stats
	s.workspaces = workspaces
}

func (s *AppState) refreshVisible() {
	indices := []int{}
	for i, item := range s.Items {
		if s.isVisible(item) {
			indices = append(indices, i)
		}
	}
	if order := s.order(); order != nil {
		slices.SortStableFunc(indices, order)
	}
	var columns [3][]int
	for _, i := range indices {
		col := s.Items[i].Status.Column()
		columns[col] = append(columns[col], i)
	}
	s.columns = columns
	s.filtered = indices
}

func (s *AppState) order() func(a, b int) int {
	items := s.Items
	switch s.SortMode {
	case SortPriority:
		return func(a, b int) int { return cmp.Compare(items[a].Priority, items[b].Priority) }
	case SortStatus:
		return func(a, b int) int { return cmp.Compare(items[a].Status, items[b].Status) }
	case SortTitle:
		return func(a, b int) int { return cmp.Compare(lowerTitle(items[a]), lowerTitle(items[b])) }
	}
	return nil
}

func lowerTitle(it Item) string { return strings.ToLower(it.Title) }

func indexByName(workspaces []WorkspaceSummary, name string) int {
	return slices.IndexFunc(workspaces, func(w WorkspaceSummary) bool { return w.Name == name })
}

func (s *AppState) isVisible(item Item) bool {
	if item.Workspace != s.Workspace {
		return false
	}
	return !s.HasFilter || matchesQuery(item, s.Filter)
}

func matchesQuery(item Item, filter string) bool {
	query := strings.ToLower(strings.TrimSpace(filter))
	fields := []string{item.Title, item.Description, item.Priority.Label(), item.Status.Label()}
	return slices.ContainsFunc(fields, func(f string) bool {
		return strings.Contains(strings.ToLower(f), query)
	})
}
