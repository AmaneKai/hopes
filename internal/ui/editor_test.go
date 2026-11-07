package ui

import (
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

func stateWith(items ...domain.Item) *domain.AppState {
	return domain.NewAppState(items, gitsync.Status{Kind: gitsync.Disabled})
}

func openedEditor(s *domain.AppState, st domain.Status) *editor {
	e := newEditor()
	e.update(OpenCreateModal{st}, s)
	return e
}

func feed(e *editor, s *domain.AppState, keys ...Key) Action {
	var last Action
	for _, k := range keys {
		last, _ = e.handleKey(k, s)
	}
	return last
}

func typed(e *editor, s *domain.AppState, text string) {
	for _, r := range text {
		feed(e, s, Rune(r))
	}
}

func draftOf(t *testing.T, act Action) TaskDraft {
	t.Helper()
	save, ok := act.(SaveTask)
	expectTrue(t, ok, "expected SaveTask")
	return save.Draft
}

func TestEscLeavesInsertBeforeCancelling(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	typed(e, s, "draft")
	expect(t, feed(e, s, esc), nil)
	expectTrue(t, !e.TextInput().Insert, "should be in normal mode")
	expect[Action](t, feed(e, s, esc), CloseModal{})
}

func TestEscOnThePriorityFieldClosesAtOnce(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	feed(e, s, tab, tab)
	expectTrue(t, e.TextInput() == nil, "priority has no text input")
	expect[Action](t, feed(e, s, esc), CloseModal{})
}

func TestTabbingBetweenFieldsResetsToInsertAtTheEnd(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	feed(e, s, tab)
	typed(e, s, "notes")
	feed(e, s, esc, tab, Special(KeyBackTab))
	expectTrue(t, e.TextInput().Insert, "should be inserting")
	expect(t, e.TextInput().Cursor, 5)
}

func TestSavingNeedsATitleAndCarriesTheFormValues(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.InProgress)
	expect(t, feed(e, s, enter), nil)
	typed(e, s, "  ship it  ")
	d := draftOf(t, feed(e, s, tab, tab, Rune('1'), enter))
	expect(t, d.ID, newTask)
	expect(t, d.Title, "ship it")
	expect(t, d.Priority, domain.Urgent)
	expect(t, d.Workspace, "Inbox")
	expect(t, d.Status, domain.InProgress)
}

func TestEnterInTheNotesFieldAddsALine(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	typed(e, s, "title")
	feed(e, s, tab)
	typed(e, s, "one")
	expect(t, feed(e, s, enter), nil)
	typed(e, s, "two")
	expect(t, draftOf(t, feed(e, s, CtrlRune('s'))).Description, "one\ntwo")
}

func TestEditingAnExistingTaskLoadsItsFields(t *testing.T) {
	task := domain.NewItem("Existing", "Work")
	task.Description, task.Priority = "details", domain.High
	s := stateWith(task)
	e := newEditor()
	e.update(OpenEditModal{0}, s)
	d := draftOf(t, feed(e, s, enter))
	expect(t, d.ID, 0)
	expect(t, d.Title+"|"+d.Description+"|"+d.Workspace, "Existing|details|Work")
	expect(t, d.Priority, domain.High)
}

func TestReopeningTheFormStartsBlank(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	typed(e, s, "leftover")
	e.update(OpenCreateModal{domain.Todo}, s)
	expect(t, feed(e, s, enter), nil)
}

func TestCtrlCCancelsFromAnyField(t *testing.T) {
	s := stateWith()
	e := openedEditor(s, domain.Todo)
	typed(e, s, "abc")
	expect[Action](t, feed(e, s, CtrlRune('c')), CloseModal{})
}

var down = Special(KeyDown)

func workspacesState() *domain.AppState {
	return stateWith(
		domain.NewItem("t", "Projects"),
		domain.NewItem("t", "Personal"),
		domain.NewItem("t", "Design"),
	)
}

func atWorkspace(s *domain.AppState) *editor {
	e := openedEditor(s, domain.Todo)
	typed(e, s, "title")
	feed(e, s, tab, tab, tab, CtrlRune('u'))
	return e
}

func savedWorkspace(t *testing.T, e *editor, s *domain.AppState) string {
	t.Helper()
	return draftOf(t, feed(e, s, enter)).Workspace
}

func TestANewTaskDefaultsToTheCurrentWorkspace(t *testing.T) {
	s := workspacesState()
	s.Workspace = "Personal"
	e := openedEditor(s, domain.Todo)
	typed(e, s, "title")
	expect(t, savedWorkspace(t, e, s), "Personal")
}

func TestWorkspaceCompletion(t *testing.T) {
	cases := []struct {
		typed string
		keys  []Key
		want  string
	}{
		{"pers", []Key{tab}, "Personal"},
		{"des", []Key{enter}, "Design"},
		{"", []Key{down, down, down, down, enter}, "Design"},
		{"design", nil, "Design"},
		{"Garden", nil, "Garden"},
		{"", nil, "Projects"},
	}
	for _, c := range cases {
		s := workspacesState()
		e := atWorkspace(s)
		typed(e, s, c.typed)
		feed(e, s, c.keys...)
		expect(t, savedWorkspace(t, e, s), c.want)
	}
}
