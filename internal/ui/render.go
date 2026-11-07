package ui

import "github.com/AmaneKai/hopes/internal/domain"

const (
	sidebarFrom      = 72
	sidebarWidth     = 24
	pageMarginFrom   = 40
	wideMarginLeft   = 3
	wideMarginRight  = 2
	narrowMarginSide = 1
)

func pageArea(main Rect) Rect {
	if main.W >= pageMarginFrom {
		return main.Inset(wideMarginLeft, 0, wideMarginRight, 0)
	}
	return main.Inset(narrowMarginSide, 0, narrowMarginSide, 0)
}

func (a *App) Render(w, h int) *Canvas {
	if idx, ok := a.SelectedIndex(); ok {
		a.State.Selection = idx
	} else {
		a.State.Selection = -1
	}
	c := NewCanvas(w, h)
	full := Rect{0, 0, w, h}
	screen := SplitV(full, Fill, 1, 1)
	body, status, bottom := screen[0], screen[1], screen[2]
	main := a.renderSidebar(c, body)
	if a.State.Focus == domain.FocusNav && main == body {
		a.nav.render(c, body, a.State, true)
	} else {
		a.renderPage(c, main)
	}
	ctx := footerContext{a.mode, a.table.Position(a.State), a.activeInput(), a.PendingChord()}
	drawStatusLine(c, status, a.State, ctx)
	drawHints(c, bottom, a.State, ctx)
	a.renderOverlay(c, full, bottom)
	return c
}

func (a *App) renderSidebar(c *Canvas, body Rect) Rect {
	if body.W < sidebarFrom {
		return body
	}
	parts := SplitH(body, 0, sidebarWidth, Fill)
	a.nav.render(c, parts[0], a.State, a.State.Focus == domain.FocusNav)
	return parts[1]
}

func (a *App) renderPage(c *Canvas, main Rect) {
	parts := SplitV(pageArea(main), 1, 1, Fill)
	drawHeader(c, parts[0], a.State)
	view := drawPageTitle(c, parts[2], a.State)
	a.view().render(c, view, a.State, a.State.Focus == domain.FocusMain)
}

func (a *App) activeInput() *TextInput {
	switch a.mode {
	case ModeCreating, ModeEditing:
		return a.editor.TextInput()
	case ModeFinding, ModeCommand:
		return a.cmdline.TextInput()
	}
	return nil
}

func (a *App) renderOverlay(c *Canvas, full, bottom Rect) {
	switch a.mode {
	case ModeCreating, ModeEditing:
		a.editor.render(c, full, a.State, true)
	case ModeFinding, ModeCommand:
		a.cmdline.render(c, bottom, a.State, true)
	case ModeHelp:
		a.help.render(c, full, a.State, true)
	}
}
