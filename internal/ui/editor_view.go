package ui

import (
	"strconv"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
)

type formRows struct {
	title, titleRule, notes, notesRule, priority, workspace, chips, hint Rect
}

func layoutForm(inner Rect) formRows {
	gap, notes := 0, 1
	if inner.H >= formHeight-2 {
		gap, notes = 1, notesRows
	}
	p := SplitV(inner, gap, 1, gap, notes, gap, gap, 1, 1, 1, Fill, 1)
	return formRows{
		title: p[1], titleRule: p[2], notes: p[3], notesRule: p[4],
		priority: p[6], workspace: p[7], chips: p[8], hint: p[10],
	}
}

func (e *editor) render(c *Canvas, area Rect, s *domain.AppState, _ bool) {
	title := map[bool]string{true: "New task", false: "Edit task"}[e.editIdx == newTask]
	inner := popupFrame(c, area, title, formWidth, formHeight)
	rows := layoutForm(inner)
	e.textRow(c, rows.title, fieldTitle, "Title", "Untitled")
	underline(c, rows.titleRule, e.field == fieldTitle)
	e.textRow(c, rows.notes, fieldNotes, "Notes", "Add notes...")
	underline(c, rows.notesRule, e.field == fieldNotes)
	e.priorityRow(c, rows.priority)
	e.workspaceRow(c, rows.workspace, s)
	suggestions := e.suggestions(s)
	e.chipsRow(c, rows.chips, suggestions)
	c.Line(rows.hint, formHint(inner.W < narrowInner, len(suggestions) > 0)...)
}

func (e *editor) labelled(c *Canvas, row Rect, field formField, label string) Rect {
	focused := e.field == field
	parts := SplitH(row, 0, 2, labelWidth, Fill)
	if focused {
		c.Text(parts[0].X, row.Y, 2, "▍ ", styleGutter)
	}
	c.Text(parts[1].X, row.Y, parts[1].W, label, emphasis(focused))
	return parts[2]
}

func (e *editor) focusedInput(field formField) *TextInput {
	if e.field == field {
		return &e.input
	}
	return nil
}

func (e *editor) textRow(c *Canvas, row Rect, field formField, label, placeholder string) {
	value := e.labelled(c, row, field, label)
	drawField(c, value, e.values[field], placeholder, e.focusedInput(field))
}

func underline(c *Canvas, r Rect, focused bool) { c.Rule(r, borderStyle(focused)) }

func (e *editor) priorityRow(c *Canvas, row Rect) {
	value := e.labelled(c, row, fieldPriority, "Priority")
	spans := []Span{}
	for _, p := range domain.AllPriorities {
		st := Style{Fg: priorityColor(p)}
		if p == e.priority {
			st.Attrs = Bold | Underline
		}
		spans = append(spans, Styled(p.Glyph()+" "+p.Label(), st), Plain("   "))
	}
	c.Line(value, spans...)
}

func (e *editor) workspaceRow(c *Canvas, row Rect, s *domain.AppState) {
	value := e.labelled(c, row, fieldWorkspace, "Workspace")
	note := ""
	if typed := e.values[fieldWorkspace]; typed != "" && !hasWorkspace(s, typed) {
		note = "new workspace"
	}
	parts := SplitH(value, 0, Fill, Width(note)+1)
	c.Text(parts[1].X+1, row.Y, Width(note), note, styleMuted)
	drawField(c, parts[0], e.values[fieldWorkspace], "Inbox", e.focusedInput(fieldWorkspace))
}

func hasWorkspace(s *domain.AppState, name string) bool {
	name = strings.TrimSpace(name)
	_, ok := s.FindWorkspace(name)
	return ok || name == ""
}

func (e *editor) chipsRow(c *Canvas, row Rect, suggestions []string) {
	value := SplitH(row, 0, 2+labelWidth, Fill)[1]
	used, target := 0, e.pickTarget()
	for i, name := range suggestions {
		w := Width(name) + 3
		if i > 0 && used+w+4 > value.W {
			more := "+" + strconv.Itoa(len(suggestions)-i)
			c.Text(value.X+used, row.Y, value.W-used, more, styleMuted)
			return
		}
		st := styleChip
		if i == target {
			st = st.With(Reverse)
		}
		used += c.Text(value.X+used, row.Y, value.W-used, " "+name+" ", st) + 1
	}
}

func formHint(narrow, picking bool) []Span {
	next := map[bool]string{true: "complete", false: "next"}[picking]
	if narrow {
		return []Span{
			Styled("[^s] ", styleKey), Styled("save ", styleMuted),
			Styled("[Tab] ", styleKey), Styled(next+" ", styleMuted), Styled("[Esc]", styleKey),
		}
	}
	spans := []Span{
		Styled("[Ctrl-s] ", styleKey), Styled("save   ", styleMuted),
		Styled("[Tab] ", styleKey), Styled(next+"   ", styleMuted),
	}
	if picking {
		spans = append(spans, Styled("[↑↓] ", styleKey), Styled("pick   ", styleMuted))
	}
	return append(spans, Styled("[Esc] ", styleKey), Styled("cancel", styleMuted))
}
