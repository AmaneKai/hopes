package ui

import (
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
	te "github.com/AmaneKai/hopes/internal/textedit"
)

type formField int

const (
	fieldTitle formField = iota
	fieldNotes
	fieldPriority
	fieldWorkspace
	fieldCount
)

const (
	formWidth   = 68
	formHeight  = 14
	notesRows   = 3
	labelWidth  = 10
	narrowInner = 48
)

type formResult int

const (
	formOpen formResult = iota
	formSave
	formCancel
)

type TaskDraft struct {
	ID          int
	Title       string
	Description string
	Priority    domain.Priority
	Workspace   string
	Status      domain.Status
}

const newTask = -1

type editor struct {
	values   [fieldCount]string
	priority domain.Priority
	field    formField
	pick     int
	editIdx  int
	status   domain.Status
	input    TextInput
}

func newEditor() *editor {
	return &editor{priority: domain.Medium, pick: -1, editIdx: newTask, input: NewTextInput()}
}

func (e *editor) TextInput() *TextInput {
	if e.field == fieldPriority {
		return nil
	}
	return &e.input
}

func (e *editor) open(values [fieldCount]string, p domain.Priority, idx int, st domain.Status) {
	e.values, e.priority, e.editIdx, e.status = values, p, idx, st
	e.field = fieldTitle
	e.focusField()
}

func (e *editor) buffer() *string {
	if e.field == fieldPriority {
		return nil
	}
	return &e.values[e.field]
}

func (e *editor) focusField() {
	e.pick = -1
	if buf := e.buffer(); buf != nil {
		e.input.Focus(*buf)
		return
	}
	e.input.Focus("")
}

func (e *editor) moveField(delta int) {
	e.field = (e.field + formField(delta) + fieldCount) % fieldCount
	e.focusField()
}

func (e *editor) paste(text string) {
	if buf := e.buffer(); buf != nil {
		e.input.Paste(buf, text)
	}
}

func (e *editor) suggestions(s *domain.AppState) []string {
	ws := e.values[fieldWorkspace]
	typing := e.field == fieldWorkspace && e.input.Insert && e.input.Cursor == te.Len(ws)
	if !typing {
		return nil
	}
	typed := strings.ToLower(strings.TrimSpace(ws))
	var prefixed, containing []string
	for _, w := range s.Workspaces() {
		lower := strings.ToLower(w.Name)
		switch {
		case lower == typed:
		case strings.HasPrefix(lower, typed):
			prefixed = append(prefixed, w.Name)
		case strings.Contains(lower, typed):
			containing = append(containing, w.Name)
		}
	}
	return append(prefixed, containing...)
}

func (e *editor) pickTarget() int {
	if e.pick >= 0 || strings.TrimSpace(e.values[fieldWorkspace]) == "" {
		return e.pick
	}
	return 0
}

func (e *editor) draft(s *domain.AppState) TaskDraft {
	workspace := strings.TrimSpace(e.values[fieldWorkspace])
	if existing, ok := s.FindWorkspace(workspace); ok {
		workspace = existing
	}
	if workspace == "" {
		workspace = s.Workspace
	}
	return TaskDraft{
		ID: e.editIdx, Title: strings.TrimSpace(e.values[fieldTitle]),
		Description: strings.TrimSpace(e.values[fieldNotes]),
		Priority:    e.priority, Workspace: workspace, Status: e.status,
	}
}

func (e *editor) saveResult() formResult {
	if strings.TrimSpace(e.values[fieldTitle]) == "" {
		return formOpen
	}
	return formSave
}

func (e *editor) handleKey(k Key, s *domain.AppState) formResult {
	if k.IsCtrl('c') {
		return formCancel
	}
	if k.Code == KeyEsc {
		return e.escape()
	}
	if e.handlePicker(k, e.suggestions(s)) {
		return formOpen
	}
	if (k.Code == KeyEnter && e.field != fieldNotes) || k.IsCtrl('s') {
		return e.saveResult()
	}
	if delta := fieldStep(k, e.field == fieldNotes); delta != 0 {
		e.moveField(delta)
		return formOpen
	}
	e.edit(k)
	return formOpen
}

func (e *editor) escape() formResult {
	if buf := e.buffer(); buf != nil && e.input.HandleEsc(buf) {
		return formOpen
	}
	return formCancel
}

func fieldStep(k Key, multiline bool) int {
	arrows := !multiline
	switch {
	case k.IsCtrl('j') || k.IsCtrl('n') || k.Code == KeyTab || (arrows && k.Code == KeyDown):
		return 1
	case k.IsCtrl('k') || k.IsCtrl('p') || k.Code == KeyBackTab || (arrows && k.Code == KeyUp):
		return -1
	}
	return 0
}

func (e *editor) handlePicker(k Key, suggestions []string) bool {
	n := len(suggestions)
	if n == 0 {
		return false
	}
	switch {
	case k.Code == KeyDown:
		e.pick = min(e.pick+1, n-1)
	case k.Code == KeyUp && e.pick < 0:
		e.pick = n - 1
	case k.Code == KeyUp:
		e.pick = max(e.pick-1, 0)
	case (k.Code == KeyTab || k.Code == KeyEnter) && e.pickTarget() >= 0:
		e.values[fieldWorkspace] = suggestions[min(e.pickTarget(), n-1)]
		e.input.Cursor, e.pick = te.Len(e.values[fieldWorkspace]), -1
	default:
		return false
	}
	return true
}

func (e *editor) edit(k Key) {
	if e.field == fieldPriority {
		e.priorityKey(k)
		return
	}
	e.pick = -1
	e.input.HandleKey(e.buffer(), k, e.field == fieldNotes)
}

func (e *editor) priorityKey(k Key) {
	switch {
	case k.IsAny(' ', 'l', 'p') || k.Code == KeyRight:
		e.priority = e.priority.Next()
	case k.IsAny('h', 'P') || k.Code == KeyLeft:
		e.priority = e.priority.Prev()
	}
	if d, ok := k.Digit(); ok {
		e.priority = priorityOr(d, e.priority)
	}
}

func priorityOr(d rune, fallback domain.Priority) domain.Priority {
	if p, ok := domain.PriorityFromDigit(d); ok {
		return p
	}
	return fallback
}

func (e *editor) openCreate(workspace string, status domain.Status) {
	e.open([fieldCount]string{fieldWorkspace: workspace}, domain.Medium, newTask, status)
}

func (e *editor) openEdit(idx int, it domain.Item) {
	values := [fieldCount]string{fieldTitle: it.Title, fieldNotes: it.Description}
	values[fieldWorkspace] = it.Workspace
	e.open(values, it.Priority, idx, it.Status)
}
