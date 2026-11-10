package ui

import (
	"fmt"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
)

var everyMode = []Mode{ModeNormal, ModeCreating, ModeEditing, ModeFinding, ModeCommand, ModeHelp}

func TestRenderSurvivesEveryTerminalSizeInEveryModeAndView(t *testing.T) {
	items := append(seed(), domain.NewItem("emoji 👨‍👩‍👧 e\u0301 日本語 title that is long enough to need truncating", "Work"))
	for _, view := range []domain.ViewMode{domain.Table, domain.Kanban} {
		for _, mode := range everyMode {
			s := newSession(t, items)
			s.app.State.ViewMode = view
			s.app.mode = mode
			for w := 0; w <= 90; w += 3 {
				for h := 0; h <= 14; h++ {
					name := fmt.Sprintf("view=%v mode=%v %dx%d", view, mode, w, h)
					t.Run(name, func(t *testing.T) { s.app.Render(w, h).ANSI() })
				}
			}
		}
	}
}

func TestSelectionHighlightKeepsGraphemesTogether(t *testing.T) {
	canvas := NewCanvas(10, 1)
	drawSelectable(canvas, Rect{0, 0, 10, 1}, []rune("ae\u0301b"), 0, 1, 1)
	if got := canvas.PlainText(); got[:len("ae\u0301b")] != "ae\u0301b" {
		t.Fatalf("accent separated from its letter: %q", got)
	}
}
