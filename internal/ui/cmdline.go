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

func (m *cmdline) submit() Action {
	if !m.find {
		return RunCommand{m.buf}
	}
	return Batch{SetFilter{strings.TrimSpace(m.buf)}, CloseModal{}}
}

func (m *cmdline) handleKey(k Key, _ *domain.AppState) (Action, bool) {
	switch {
	case k.Code == KeyEnter:
		return m.submit(), true
	case k.Code == KeyEsc && !m.input.HandleEsc(&m.buf):
		return CloseModal{}, true
	case k.Code != KeyEsc:
		m.input.HandleKey(&m.buf, k, false)
	}
	return nil, true
}

func (m *cmdline) update(act Action, s *domain.AppState) {
	switch act.(type) {
	case OpenCommand:
		m.find, m.buf = false, ""
	case OpenFind:
		m.find, m.buf = true, s.Filter
	default:
		return
	}
	m.input.Focus(m.buf)
}

func (m *cmdline) render(c *Canvas, r Rect, _ *domain.AppState, _ bool) {
	c.Fill(r, Style{})
	prefix := map[bool]string{true: "/", false: ":"}[m.find]
	c.Text(r.X, r.Y, 1, prefix, styleNone)
	drawField(c, r.Inset(1, 0, 0, 0), m.buf, "", &m.input)
}
