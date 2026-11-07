package ui

import (
	"fmt"

	"github.com/AmaneKai/hopes/internal/domain"
)

const (
	helpKeyWidth = 19
	helpWidth    = 64
)

type helpSection struct {
	title string
	rows  []hint
}

var helpSections = []helpSection{
	{"Workspaces", []hint{
		{"Tab / Shift-Tab", "Next / previous workspace"},
		{"h / Ctrl-w", "Focus the workspace list"},
		{"1 .. 9", "Jump to a workspace (list focused)"},
		{":ws name", "Open or create a workspace"},
		{":rename name", "Rename this workspace"},
	}},
	{"Navigation", []hint{
		{"j / k", "Move down / up"},
		{"gg / G", "Jump to top / bottom"},
		{"Ctrl-d / Ctrl-u", "Page down / up"},
		{"v", "Switch table / board"},
		{"s", "Cycle sort"},
	}},
	{"Search", []hint{
		{"/", "Find in this workspace"},
		{"Esc", "Clear the search"},
		{":", "Command line"},
	}},
	{"Tasks", []hint{
		{"Space", "Toggle status"},
		{"a / o / i", "Add task"},
		{"e / c / Enter", "Edit task"},
		{"dd / x", "Delete task"},
		{"u", "Undo delete"},
		{"J / K", "Move task down / up"},
		{"p / P", "Next / previous priority"},
		{"1 2 3 4", "Urgent / High / Medium / Low"},
		{"S / F5", "Sync now"},
	}},
	{"Editing", []hint{
		{"Tab / Ctrl-j", "Next field"},
		{"Shift-Tab / Ctrl-k", "Previous field"},
		{"Workspace field", "Tab completes; a new name creates one"},
		{"Ctrl-s", "Save"},
		{"Esc", "Normal mode, then cancel"},
		{"h j k l w b e", "Move"},
		{"0 / $", "Line start / end"},
		{"i a I A o", "Insert"},
		{"v", "Visual selection"},
		{"x dd D", "Delete"},
		{"y p P", "Yank / paste"},
		{"Ctrl-w / Ctrl-u", "Delete word / clear"},
	}},
	{"General", []hint{{"? / q / Esc", "Close help"}, {"Ctrl-c", "Cancel form"}}},
}

func helpLines() [][]Span {
	var lines [][]Span
	for i, section := range helpSections {
		if i > 0 {
			lines = append(lines, nil)
		}
		lines = append(lines, []Span{Styled(section.title, styleSection)})
		for _, row := range section.rows {
			key := fmt.Sprintf("%-*s", helpKeyWidth, row.key)
			lines = append(lines, []Span{Styled(key, styleKey), Styled(row.desc, styleMuted)})
		}
	}
	return lines
}

type helpPopup struct {
	scroll    int
	maxScroll int
}

func (h *helpPopup) handleKey(k Key, _ *domain.AppState) (Action, bool) {
	switch {
	case k.Code == KeyEsc || k.Code == KeyEnter || k.IsAny('?', 'q', ' '):
		return CloseModal{}, true
	case k.Is('j') || k.Code == KeyDown:
		h.scroll = min(h.scroll+1, h.maxScroll)
	case k.Is('k') || k.Code == KeyUp:
		h.scroll = max(h.scroll-1, 0)
	}
	return nil, true
}

func (h *helpPopup) update(act Action, _ *domain.AppState) {
	switch act.(type) {
	case OpenHelp, CloseModal:
		h.scroll = 0
	}
}

func (h *helpPopup) render(c *Canvas, area Rect, _ *domain.AppState, _ bool) {
	lines := helpLines()
	inner := popupFrame(c, area, "Keyboard shortcuts", helpWidth, len(lines)+4)
	parts := SplitV(inner, Fill, 1, 1)
	h.maxScroll = max(len(lines)-parts[0].H, 0)
	h.scroll = min(h.scroll, h.maxScroll)
	c.Lines(parts[0], lines[h.scroll:])
	c.Line(parts[2],
		Styled("[j/k] ", styleKey), Styled("scroll   ", styleMuted),
		Styled("[Esc] ", styleKey), Styled("close", styleMuted),
	)
}
