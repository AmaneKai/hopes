package ui

import (
	"fmt"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

func drawHeader(c *Canvas, r Rect, s *domain.AppState) {
	left := []Span{Styled("Hopes / ", styleMuted), Styled(s.Workspace, styleBold)}
	if n := len(s.Workspaces()); n > 1 {
		left = append(left, Styled(fmt.Sprintf("  %d/%d", s.WorkspacePosition()+1, n), styleMuted))
	}
	c.Line(r, left...)
	if sync, ok := syncSpan(s.SyncStatus); ok {
		w := Width(sync.Text)
		c.Text(r.X+r.W-w, r.Y, w, sync.Text, sync.Style)
	}
}

var syncLabels = map[gitsync.Kind]Span{
	gitsync.Idle:              Styled("Idle", styleMuted),
	gitsync.Synced:            Styled("Synced", styleMuted),
	gitsync.UpdatedRemoteData: Styled("Synced", styleMuted),
	gitsync.Offline:           Styled("Offline", styleMuted),
	gitsync.Conflict:          Styled("Conflict", Style{Fg: red, Attrs: Bold}),
	gitsync.Failed:            Styled("Sync error", Style{Fg: red, Attrs: Bold}),
}

func syncSpan(status gitsync.Status) (Span, bool) {
	label, ok := syncLabels[status.Kind]
	if status.Kind == gitsync.Syncing {
		label, ok = Styled(status.Detail, Style{Fg: yellow}), true
	}
	label.Text = "☁ " + label.Text
	return label, ok
}

func drawPageTitle(c *Canvas, r Rect, s *domain.AppState) Rect {
	parts := SplitV(r, 1, 1, 1, Fill)
	c.Text(parts[0].X, parts[0].Y, parts[0].W, s.Workspace, styleBold)
	c.Line(parts[1], summary(s)...)
	return parts[3]
}

func summary(s *domain.AppState) []Span {
	stats := s.Stats()
	dot := Styled("  ·  ", styleGhost)
	spans := []Span{Styled("Empty", styleMuted)}
	if stats.Total > 0 {
		noun := pick(stats.Total == 1, "task", "tasks")
		spans = []Span{
			Styled(fmt.Sprintf("%d %s", stats.Total, noun), styleMuted), dot,
			Styled(fmt.Sprintf("%d open", stats.Todo+stats.InProgress), styleMuted), dot,
			Styled(fmt.Sprintf("%d%% done", stats.Percent), styleMuted),
		}
	}
	if s.HasFilter {
		found := fmt.Sprintf("%d matching %s", len(s.FilteredIndices()), quote(s.Filter))
		spans = append(spans, dot, Styled(found, styleStatusMsg))
	}
	return spans
}

type footerContext struct {
	mode     Mode
	position int
	input    *TextInput
	pending  string
}

var modeLabels = map[Mode]Span{
	ModeNormal:   Styled("  NORMAL", Style{Fg: gray, Attrs: Bold}),
	ModeCreating: Styled("  INSERT", Style{Fg: cyan, Attrs: Bold}),
	ModeEditing:  Styled("  EDIT", Style{Fg: magenta, Attrs: Bold}),
	ModeFinding:  Styled("  FIND", Style{Fg: yellow, Attrs: Bold}),
	ModeCommand:  Styled("  COMMAND", styleBold),
	ModeHelp:     Styled("  HELP", Style{Fg: blue, Attrs: Bold}),
}

func inputBadge(t *TextInput) Span {
	switch {
	case t.Insert:
		return Styled("-- INSERT --", Style{Fg: green, Attrs: Bold})
	case t.Visual:
		return Styled("-- VISUAL --", Style{Fg: magenta, Attrs: Bold})
	}
	return Styled("-- NORMAL --", Style{Fg: yellow, Attrs: Bold})
}

func drawStatusLine(c *Canvas, r Rect, s *domain.AppState, ctx footerContext) {
	c.Fill(r, styleStatusBar)
	right := fmt.Sprintf("  %d / %d  ", ctx.position, len(s.FilteredIndices()))
	parts := SplitH(r, 0, Fill, Width(right))
	c.Text(parts[1].X, r.Y, parts[1].W, right, styleMuted)
	spans := []Span{modeLabels[ctx.mode], Plain(" ")}
	if ctx.input != nil {
		spans = append(spans, inputBadge(ctx.input), Plain(" "))
	}
	if ctx.pending != "" {
		spans = append(spans, Styled(ctx.pending, styleStatusMsg), Plain(" "))
	}
	c.Line(parts[0], append(spans, Styled(s.Status(), styleStatusMsg))...)
}

type hint struct{ key, desc string }

var focusHints = map[domain.FocusPane][]hint{
	domain.FocusMain: {{"a", "add"}, {"Space", "toggle"}, {"Tab", "next workspace"}},
	domain.FocusNav:  {{"j/k", "switch"}, {"1-9", "jump"}, {"Enter", "open"}},
}

var (
	insertHints = []hint{
		{"<Tab>/<C-j/k>", "Next Field / Complete"}, {"<Enter>/<C-s>", "Save"},
		{"<C-w>", "Del Word"}, {"<C-u>", "Clear"}, {"<Esc>", "Normal Mode / Cancel"},
	}
	normalHints = []hint{
		{"h/j/k/l", "Move"}, {"w/b/e", "Word"}, {"0/$", "Line"}, {"v", "Visual"},
		{"x/dd/D", "Delete"}, {"p/P", "Paste"}, {"i/a/I/A/o", "Insert"}, {"<Esc>", "Cancel"},
	}
	visualHints = []hint{
		{"h/j/k/l/w/b/e/0/$", "Extend"}, {"d/x", "Cut"}, {"y", "Yank"},
		{"c", "Change"}, {"v / <Esc>", "Exit Visual"},
	}
)

func inlineHints(hints []hint) []Span {
	spans := []Span{}
	for _, h := range hints {
		spans = append(spans, Styled(h.key, styleKey), Styled(":"+h.desc+" ", styleMuted))
	}
	return spans
}

func bracketHints(hints []hint) []Span {
	spans := []Span{}
	for _, h := range hints {
		spans = append(spans, Styled("["+h.key+"] ", styleKey), Styled(h.desc+"  ", styleMuted))
	}
	return spans
}

func spansWidth(spans []Span) int {
	w := 0
	for _, s := range spans {
		w += Width(s.Text)
	}
	return w
}

func drawHints(c *Canvas, r Rect, s *domain.AppState, ctx footerContext) {
	switch ctx.mode {
	case ModeNormal:
		drawNormalHints(c, r, s)
	case ModeCreating, ModeEditing:
		c.Line(r, editingHints(ctx.input)...)
	case ModeHelp:
		c.Line(r, Styled("  Esc ", styleKey), Styled("close", styleMuted))
	}
}

func drawNormalHints(c *Canvas, r Rect, s *domain.AppState) {
	left := []Span{
		Styled("  ? ", styleKey), Styled("help   ", styleMuted),
		Styled(": ", styleKey), Styled("commands   ", styleMuted),
		Styled("/ ", styleKey), Styled("find", styleMuted),
	}
	used := c.Line(r, left...)
	right := bracketHints(focusHints[s.Focus])
	if w := spansWidth(right); used+w <= r.W {
		c.Line(Rect{r.X + r.W - w, r.Y, w, 1}, right...)
	}
}

func editingHints(input *TextInput) []Span {
	switch {
	case input == nil || input.Insert:
		return inlineHints(insertHints)
	case input.Visual:
		return inlineHints(visualHints)
	}
	return inlineHints(normalHints)
}
