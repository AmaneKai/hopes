package ui

import "github.com/AmaneKai/hopes/internal/domain"

type taskView interface {
	handleKey(k Key, a *App) bool
	render(c *Canvas, r Rect, s *domain.AppState, focused bool)
	selectedIndex(s *domain.AppState) (int, bool)
	pendingLabel() string
	clearPending()
}

func pick[T any](cond bool, yes, no T) T {
	if cond {
		return yes
	}
	return no
}

type chords struct{ g, d bool }

func (c *chords) pressG() bool {
	done := c.g
	c.g, c.d = !c.g, false
	return done
}

func (c *chords) pressD() bool {
	done := c.d
	c.d, c.g = !c.d, false
	return done
}

func (c *chords) clear() { c.g, c.d = false, false }

func (c *chords) label() string {
	switch {
	case c.g:
		return "g..."
	case c.d:
		return "d..."
	}
	return ""
}

func taskCommand(k Key, a *App) func(int) {
	switch {
	case k.IsAny('e', 'c') || k.Code == KeyEnter:
		return a.openEdit
	case k.Is('x'):
		return a.deleteTask
	case k.IsAny('J', 'K'):
		up := k.Is('K')
		return func(i int) { a.moveTask(i, up) }
	case k.IsAny('p', 'P'):
		forward := k.Is('p')
		return func(i int) { a.cyclePriority(i, forward) }
	}
	return priorityCommand(k, a)
}

func priorityCommand(k Key, a *App) func(int) {
	d, isDigit := k.Digit()
	if !isDigit {
		return nil
	}
	p, valid := domain.PriorityFromDigit(d)
	if !valid {
		return nil
	}
	return func(i int) { a.setPriority(i, p) }
}

func runOnSelected(cmd func(int), idx int, ok bool) {
	if ok {
		cmd(idx)
	}
}

func runTaskCommand(k Key, a *App, idx int, ok bool) bool {
	cmd := taskCommand(k, a)
	if cmd == nil {
		return false
	}
	runOnSelected(cmd, idx, ok)
	return true
}

func scrollOffset(selected, length, rows, offset, pad int) int {
	if rows <= 0 || length <= rows {
		return 0
	}
	pad = min(pad, (rows-1)/2)
	maxOffset := length - rows
	offset = min(offset, maxOffset)
	switch {
	case selected < offset+pad:
		offset = max(selected-pad, 0)
	case selected+pad >= offset+rows:
		offset = selected + pad + 1 - rows
	}
	return max(min(offset, maxOffset), 0)
}
