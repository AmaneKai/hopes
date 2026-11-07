package ui

import (
	"strconv"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
)

const boardGap = 2

type boardView struct {
	col    int
	cursor [3]int
	scroll [3]int
	chords chords
}

func (b *boardView) status() domain.Status { return domain.StatusFromColumn(b.col) }

func (b *boardView) columnLen(s *domain.AppState) int { return len(s.KanbanColumn(b.status())) }

func (b *boardView) selectedIndex(s *domain.AppState) (int, bool) {
	items := s.KanbanColumn(b.status())
	if len(items) == 0 {
		return 0, false
	}
	return items[min(b.cursor[b.col], len(items)-1)], true
}

func (b *boardView) pendingLabel() string { return b.chords.label() }
func (b *boardView) clearPending()        { b.chords.clear() }

func (b *boardView) moveCard(delta, count int) {
	if count > 0 {
		b.cursor[b.col] = max(min(b.cursor[b.col]+delta, count-1), 0)
	}
}

func (b *boardView) moveColumn(delta int) { b.col = max(min(b.col+delta, 2), 0) }

func (b *boardView) handleKey(k Key, s *domain.AppState) (Action, bool) {
	count := b.columnLen(s)
	idx, ok := b.selectedIndex(s)
	if act, handled := taskCommand(k, idx, ok); handled {
		return act, true
	}
	switch {
	case k.Is('j') || k.Code == KeyDown:
		b.moveCard(1, count)
	case k.Is('k') || k.Code == KeyUp:
		b.moveCard(-1, count)
	case k.Is('h') || k.Code == KeyLeft:
		b.moveColumn(-1)
	case k.Is('l') || k.Code == KeyRight:
		b.moveColumn(1)
	case k.Is('G') || k.Code == KeyEnd:
		b.moveCard(count, count)
	case k.Code == KeyHome || (k.Is('g') && b.chords.pressG()):
		b.cursor[b.col] = 0
	case k.Is('g'):
	default:
		return b.cardKey(k, idx, ok)
	}
	return nil, true
}

func (b *boardView) cardKey(k Key, idx int, ok bool) (Action, bool) {
	switch {
	case k.Is('d') && b.chords.pressD():
		return onSelected(idx, ok, func(i int) Action { return DeleteTask{i} })
	case k.Is('d'):
		return nil, true
	case k.IsAny('H', 'L', ' '):
		forward := !k.Is('H')
		return onSelected(idx, ok, func(i int) Action { return ShiftTaskStatus{i, forward} })
	case k.IsAny('a', 'o', 'i'):
		return OpenCreateModal{b.status()}, true
	}
	return nil, false
}

func (b *boardView) update(act Action, s *domain.AppState) {
	if s.ViewMode != domain.Kanban {
		return
	}
	switch act := act.(type) {
	case ShiftTaskStatus:
		b.moveColumn(map[bool]int{true: 1, false: -1}[act.Forward])
	case SelectNext:
		b.moveCard(1, b.columnLen(s))
	case SelectPrev:
		b.moveCard(-1, b.columnLen(s))
	}
}

func (b *boardView) render(c *Canvas, r Rect, s *domain.AppState, focused bool) {
	for i, area := range SplitH(r, boardGap, Fill, Fill, Fill) {
		b.renderColumn(c, area, s, i, focused && b.col == i)
	}
}

func (b *boardView) renderColumn(c *Canvas, r Rect, s *domain.AppState, col int, active bool) {
	status := domain.StatusFromColumn(col)
	indices := s.KanbanColumn(status)
	parts := SplitV(r, 1, 1, 1, Fill)
	c.Line(parts[0],
		Styled(status.Glyph()+" ", Style{Fg: statusColor(status)}),
		Styled(status.BoardLabel(), emphasis(active)),
		Styled("  "+strconv.Itoa(len(indices)), styleMuted),
	)
	c.Rule(parts[1], borderStyle(active))
	body := parts[3]
	if len(indices) == 0 {
		c.Text(body.X, body.Y, body.W, emptyColumn[status], styleGhost)
		return
	}
	b.renderCards(c, body, s, col, active)
}

var emptyColumn = map[domain.Status]string{
	domain.Todo:       "Nothing to do. Press a to add",
	domain.InProgress: "Nothing in progress. Space starts one",
	domain.Complete:   "Nothing completed yet",
}

func (b *boardView) renderCards(c *Canvas, body Rect, s *domain.AppState, col int, active bool) {
	indices := s.KanbanColumn(domain.StatusFromColumn(col))
	heights := make([]int, len(indices))
	for i, idx := range indices {
		heights[i] = cardHeight(s.Items[idx])
	}
	selected := min(b.cursor[col], len(indices)-1)
	b.scroll[col] = firstCard(heights, selected, b.scroll[col], body.H)
	y := body.Y
	for pos := b.scroll[col]; pos < len(indices) && y+heights[pos] <= body.Y+body.H; pos++ {
		card := Rect{body.X, y, body.W, heights[pos]}
		renderCard(c, card, s.Items[indices[pos]], active && pos == selected)
		y += heights[pos]
	}
}

func cardHeight(item domain.Item) int {
	h := 3
	if item.Description != "" {
		h++
	}
	if item.Status != domain.Complete {
		h++
	}
	return h
}

func firstCard(heights []int, selected, scroll, room int) int {
	first := min(scroll, selected)
	for first < selected && sum(heights[first:selected+1]) > room {
		first++
	}
	return first
}

func sum(values []int) int {
	total := 0
	for _, v := range values {
		total += v
	}
	return total
}

func renderCard(c *Canvas, r Rect, item domain.Item, selected bool) {
	c.Box(r, borderStyle(selected))
	inner := r.Inset(2, 1, 2, 1)
	lines := [][]Span{{Styled(Ellipsize(item.Title, inner.W), titleStyle(item, selected))}}
	if item.Description != "" {
		first, _, _ := strings.Cut(item.Description, "\n")
		lines = append(lines, []Span{Styled(domain.Snippet(first, max(inner.W-3, 0)), styleMuted)})
	}
	if item.Status != domain.Complete {
		label := item.Priority.Glyph() + " " + item.Priority.Label()
		lines = append(lines, []Span{Styled(label, Style{Fg: priorityColor(item.Priority)})})
	}
	c.Lines(inner, lines)
}
