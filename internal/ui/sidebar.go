package ui

import (
	"fmt"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
)

const (
	sidebarPad    = 2
	sidebarChrome = 5
	sidebarInset  = 1
)

type sidebar struct {
	offset int
}

func jumpTo(s *domain.AppState, index int) (Action, bool) {
	ws := s.Workspaces()
	if index < 0 || index >= len(ws) || ws[index].Name == s.Workspace {
		return nil, true
	}
	return SwitchWorkspace{ws[index].Name}, true
}

func (n *sidebar) handleKey(k Key, s *domain.AppState) (Action, bool) {
	here, last := s.WorkspacePosition(), len(s.Workspaces())-1
	switch {
	case k.Is('j') || k.Code == KeyDown:
		return jumpTo(s, min(here+1, last))
	case k.Is('k') || k.Code == KeyUp:
		return jumpTo(s, max(here-1, 0))
	case k.Is('g') || k.Code == KeyHome:
		return jumpTo(s, 0)
	case k.Is('G') || k.Code == KeyEnd:
		return jumpTo(s, last)
	case k.IsAny('l', ' ') || k.Code == KeyEnter || k.Code == KeyRight:
		return SetFocus{domain.FocusMain}, true
	}
	if d, ok := k.Digit(); ok && d != '0' {
		return jumpTo(s, int(d-'1'))
	}
	return nil, false
}

func (n *sidebar) update(Action, *domain.AppState) {}

func (n *sidebar) render(c *Canvas, r Rect, s *domain.AppState, focused bool) {
	c.Fill(r, styleSidebar)
	inner := r.Inset(1, 0, 1, 0)
	ws := s.Workspaces()
	here := s.WorkspacePosition()
	rows := max(inner.H-sidebarChrome, 1)
	n.offset = scrollOffset(here, len(ws), rows, n.offset, sidebarPad)
	countWidth := 1
	for _, w := range ws {
		countWidth = max(countWidth, len(fmt.Sprint(w.Open)))
	}
	pad := strings.Repeat(" ", sidebarInset)
	c.Line(inner.Row(2), Styled(pad+"Workspaces", styleSection))
	shown := min(len(ws)-n.offset, rows)
	for i := range shown {
		row := inner.Row(3 + i)
		pos := n.offset + i
		if pos == here {
			c.Fill(row, rowStyle(focused))
		}
		inset := row.Inset(sidebarInset, 0, sidebarInset, 0)
		renderWorkspace(c, inset, ws[pos], pos == here, countWidth)
	}
	c.Line(inner.Row(4+shown), Styled(pad+"+ New", styleMuted), Styled("  :ws name", styleGhost))
}

func renderWorkspace(c *Canvas, row Rect, w domain.WorkspaceSummary, current bool, countWidth int) {
	nameStyle := styleNone
	if current {
		nameStyle = styleBold
	}
	count := fmt.Sprintf("%*s", countWidth, "")
	if w.Open > 0 {
		count = fmt.Sprintf("%*d", countWidth, w.Open)
	}
	nameWidth := max(row.W-countWidth-1, 0)
	c.Text(row.X, row.Y, nameWidth, Ellipsize(w.Name, nameWidth), nameStyle)
	c.Text(row.X+row.W-countWidth, row.Y, countWidth, count, styleMuted)
}
