package ui

import (
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
)

type cmdline struct {
	find  bool
	buf   string
	input TextInput
}

func newCmdline() *cmdline { return &cmdline{input: NewTextInput()} }

func (m *cmdline) TextInput() *TextInput { return &m.input }

func (m *cmdline) paste(text string) { m.input.Paste(&m.buf, text) }

func (m *cmdline) open(find bool, text string) {
	m.find, m.buf = find, text
	m.input.Focus(text)
}

func (m *cmdline) submit(a *App) {
	if !m.find {
		a.runCommand(m.buf)
		return
	}
	a.setFilter(strings.TrimSpace(m.buf))
	a.closeModal()
}

func (m *cmdline) handleKey(k Key, a *App) {
	switch {
	case k.Code == KeyEnter:
		m.submit(a)
	case k.Code == KeyEsc && !m.input.HandleEsc(&m.buf):
		a.closeModal()
	case k.Code != KeyEsc:
		m.input.HandleKey(&m.buf, k, false)
	}
}

func (m *cmdline) render(c *Canvas, r Rect, _ *domain.AppState, _ bool) {
	c.Fill(r, Style{})
	prefix := pick(m.find, "/", ":")
	c.Text(r.X, r.Y, 1, prefix, styleNone)
	drawField(c, r.Inset(1, 0, 0, 0), m.buf, "", &m.input)
}
