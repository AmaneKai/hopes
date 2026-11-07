package ui

import (
	"github.com/AmaneKai/hopes/internal/domain"
)

const (
	tablePage    = 5
	tablePad     = 3
	statusCol    = 3
	priorityCol  = 8
	columnGap    = 1
	minTitleCols = 8
)

type tableView struct {
	selected int
	offset   int
	chords   chords
}

func newTableView(hasItems bool) *tableView {
	t := &tableView{selected: -1}
	if hasItems {
		t.selected = 0
	}
	return t
}

func (t *tableView) Position(s *domain.AppState) int {
	if t.selected < 0 || len(s.FilteredIndices()) == 0 {
		return 0
	}
	return t.selected + 1
}

func (t *tableView) selectedIndex(s *domain.AppState) (int, bool) {
	visible := s.FilteredIndices()
	if t.selected < 0 || t.selected >= len(visible) {
		return 0, false
	}
	return visible[t.selected], true
}

func (t *tableView) pendingLabel() string { return t.chords.label() }
func (t *tableView) clearPending()        { t.chords.clear() }

func (t *tableView) moveTo(row, count int) {
	if count == 0 {
		t.selected = -1
		return
	}
	t.selected = max(min(row, count-1), 0)
}

func (t *tableView) step(delta, count int) { t.moveTo(max(t.selected, 0)+delta, count) }

func (t *tableView) clamp(s *domain.AppState) { t.moveTo(t.selected, len(s.FilteredIndices())) }

func (t *tableView) handleKey(k Key, a *App) bool {
	count := len(a.State.FilteredIndices())
	idx, ok := t.selectedIndex(a.State)
	if runTaskCommand(k, a, idx, ok) {
		return true
	}
	switch {
	case k.IsCtrl('d') || k.Code == KeyPageDown:
		t.step(tablePage, count)
	case k.IsCtrl('u') || k.Code == KeyPageUp:
		t.step(-tablePage, count)
	case k.Is('j') || k.Code == KeyDown:
		t.step(1, count)
	case k.Is('k') || k.Code == KeyUp:
		t.step(-1, count)
	case k.IsAny('G', 'L') || k.Code == KeyEnd:
		t.moveTo(count-1, count)
	case k.Is('H') || k.Code == KeyHome || (k.Is('g') && t.chords.pressG()):
		t.moveTo(0, count)
	case k.Is('g'):
	default:
		return t.taskKey(k, a, idx, ok)
	}
	return true
}

func (t *tableView) taskKey(k Key, a *App, idx int, ok bool) bool {
	switch {
	case k.Is('d') && t.chords.pressD():
		runOnSelected(a.deleteTask, idx, ok)
	case k.Is('d'):
	case k.IsAny('a', 'o', 'i'):
		a.openCreate(domain.Todo)
	case k.Is(' '):
		runOnSelected(a.toggleStatus, idx, ok)
	default:
		return false
	}
	return true
}

func tableColumns(r Rect) []Rect {
	return SplitH(r, columnGap, statusCol, Fill, priorityCol)
}

func (t *tableView) render(c *Canvas, r Rect, s *domain.AppState, focused bool) {
	parts := SplitV(r, 1, 1, Fill)
	head, rule, body := parts[0], parts[1], parts[2]
	cols := tableColumns(head)
	c.Text(cols[1].X, head.Y, cols[1].W, "Title", styleMuted)
	c.Text(cols[2].X, head.Y, cols[2].W, "Priority", styleMuted)
	c.Rule(rule, styleBorder)

	visible := s.FilteredIndices()
	if len(visible) == 0 {
		c.Lines(body, emptyTable(s))
		return
	}
	t.offset = scrollOffset(t.selected, len(visible), body.H, t.offset, tablePad)
	shown := min(len(visible)-t.offset, body.H)
	for i := range shown {
		row := body.Row(i)
		if t.offset+i == t.selected {
			c.Fill(row, rowStyle(focused))
		}
		renderRow(c, row, s.Items[visible[t.offset+i]])
	}
	if shown < body.H {
		c.Line(body.Row(shown), ghostRow("+", "New")...)
	}
}

func renderRow(c *Canvas, row Rect, item domain.Item) {
	cols := tableColumns(row)
	glyph := Style{Fg: statusColor(item.Status)}
	c.Text(cols[0].X, row.Y, cols[0].W, " "+item.Status.Glyph(), glyph)
	c.Text(cols[1].X, row.Y, cols[1].W, Ellipsize(item.Title, cols[1].W), titleStyle(item, false))
	if item.Status != domain.Complete {
		label := item.Priority.Glyph() + " " + item.Priority.ShortLabel()
		c.Text(cols[2].X, row.Y, cols[2].W, label, priorityStyle(item.Priority))
	}
}

func ghostRow(glyph, title string) []Span {
	lead := " " + glyph + "  "
	return []Span{Styled(lead, styleGhost), Styled(title, styleGhost)}
}

func emptyTable(s *domain.AppState) [][]Span {
	lines := [][]Span{ghostRow("○", "Untitled task"), ghostRow("○", "Untitled task"), nil}
	if s.Stats().Total == 0 {
		return append(lines, []Span{
			Plain("    "), Styled("a ", styleKey), Styled("add a task to ", styleMuted),
			Styled(s.Workspace, styleBold),
		})
	}
	return append(lines, []Span{
		Plain("    "), Styled("No tasks match.  ", styleMuted),
		Styled("Esc ", styleKey), Styled("clears the search", styleMuted),
	})
}
