package ui

import "github.com/AmaneKai/hopes/internal/domain"

var (
	darkGray = Ansi(8)
	red      = Ansi(1)
	green    = Ansi(2)
	yellow   = Ansi(3)
	blue     = Ansi(4)
	magenta  = Ansi(5)
	cyan     = Ansi(6)
	gray     = Ansi(7)
	lightRed = Ansi(9)
)

var (
	styleNone        = Style{}
	styleBold        = Style{Attrs: Bold}
	styleMuted       = Style{Fg: darkGray}
	styleGhost       = Style{Fg: darkGray, Attrs: Dim}
	styleSection     = Style{Fg: darkGray, Attrs: Bold}
	styleBorder      = Style{Fg: darkGray}
	styleBorderFocus = Style{}
	styleCompleted   = Style{Fg: darkGray, Attrs: Strike}
	styleSidebar     = Style{Bg: Ansi(234)}
	styleRowActive   = Style{Bg: Ansi(238)}
	styleRowInactive = Style{Bg: Ansi(236)}
	styleStatusBar   = Style{Bg: Ansi(236)}
	styleStatusMsg   = Style{Fg: yellow}
	styleKey         = Style{Attrs: Bold}
	styleChip        = Style{Fg: cyan}
	styleSelection   = Style{Attrs: Reverse}
	styleGutter      = Style{Fg: cyan}
)

func rowStyle(focused bool) Style {
	if focused {
		return styleRowActive
	}
	return styleRowInactive
}

func emphasis(focused bool) Style {
	if focused {
		return styleBold
	}
	return styleMuted
}

func borderStyle(focused bool) Style {
	if focused {
		return styleBorderFocus
	}
	return styleBorder
}

func priorityColor(p domain.Priority) Color {
	return [...]Color{lightRed, red, yellow, darkGray}[p]
}

func priorityStyle(p domain.Priority) Style {
	st := Style{Fg: priorityColor(p)}
	if p == domain.Urgent || p == domain.High {
		st.Attrs = Bold
	}
	return st
}

func statusColor(s domain.Status) Color {
	return [...]Color{darkGray, cyan, green}[s]
}

func titleStyle(item domain.Item, selected bool) Style {
	switch {
	case item.Status == domain.Complete:
		return styleCompleted
	case selected:
		return styleBold
	}
	return styleNone
}
