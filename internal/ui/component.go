package ui

import "github.com/AmaneKai/hopes/internal/domain"

type component interface {
	handleKey(k Key, s *domain.AppState) (Action, bool)
	update(a Action, s *domain.AppState)
	render(c *Canvas, r Rect, s *domain.AppState, focused bool)
}

type taskView interface {
	component
	selectedIndex(s *domain.AppState) (int, bool)
	pendingLabel() string
	clearPending()
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

func onSelected(idx int, ok bool, build func(int) Action) (Action, bool) {
	if !ok {
		return nil, true
	}
	return build(idx), true
}

func taskCommand(k Key, idx int, ok bool) (Action, bool) {
	switch {
	case k.IsAny('e', 'c') || k.Code == KeyEnter:
		return onSelected(idx, ok, func(i int) Action { return OpenEditModal{i} })
	case k.Is('x'):
		return onSelected(idx, ok, func(i int) Action { return DeleteTask{i} })
	case k.IsAny('J', 'K'):
		up := k.Is('K')
		return onSelected(idx, ok, func(i int) Action { return MoveTask{i, up} })
	case k.IsAny('p', 'P'):
		forward := k.Is('p')
		return onSelected(idx, ok, func(i int) Action { return CyclePriority{i, forward} })
	}
	if d, isDigit := k.Digit(); isDigit {
		return setPriority(d, idx, ok)
	}
	return nil, false
}

func setPriority(d rune, idx int, ok bool) (Action, bool) {
	p, valid := domain.PriorityFromDigit(d)
	if !valid {
		return nil, false
	}
	return onSelected(idx, ok, func(i int) Action { return SetPriority{i, p} })
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
