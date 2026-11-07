package ui

import (
	"time"

	tea "charm.land/bubbletea/v2"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

type syncMsg gitsync.Status

type redrawMsg struct{}

type Model struct {
	App    *App
	width  int
	height int
}

func (m Model) waitForSync() tea.Cmd {
	statuses := m.App.sync.Statuses()
	return func() tea.Msg { return syncMsg(<-statuses) }
}

func (m Model) Init() tea.Cmd { return m.waitForSync() }

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width, m.height = msg.Width, msg.Height
	case tea.KeyPressMsg:
		m.App.HandleKey(KeyFromTea(msg))
	case tea.PasteMsg:
		m.App.Paste(msg.Content)
	case syncMsg:
		m.App.ApplySyncStatus(gitsync.Status(msg))
		cmd = m.waitForSync()
	}
	if m.App.ShouldQuit() {
		return m, tea.Quit
	}
	return m, tea.Batch(cmd, m.expireStatus())
}

func (m Model) expireStatus() tea.Cmd {
	if m.App.State.Status() == "" {
		return nil
	}
	return tea.Tick(domain.StatusTTL, func(time.Time) tea.Msg { return redrawMsg{} })
}

func (m Model) View() tea.View {
	canvas := m.App.Render(m.width, m.height)
	v := tea.NewView(canvas.ANSI())
	v.AltScreen = true
	if canvas.Cursor != nil {
		v.Cursor = tea.NewCursor(canvas.Cursor[0], canvas.Cursor[1])
		v.Cursor.Shape = cursorShape(m.App.activeInput())
	}
	return v
}

func cursorShape(input *TextInput) tea.CursorShape {
	if input != nil && input.Insert {
		return tea.CursorBar
	}
	return tea.CursorBlock
}
