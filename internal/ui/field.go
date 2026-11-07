package ui

import (
	"strings"

	te "github.com/AmaneKai/hopes/internal/textedit"
)

func popupFrame(c *Canvas, area Rect, title string, w, h int) Rect {
	r := area.Centered(w, h)
	c.Fill(r, Style{})
	c.Box(r, styleBorder)
	c.Text(r.X+1, r.Y, r.W-2, " "+title+" ", styleBold)
	return r.Inset(2, 1, 2, 1)
}

func drawField(c *Canvas, r Rect, value, placeholder string, input *TextInput) {
	if value == "" {
		c.Text(r.X, r.Y, r.W, placeholder, styleMuted)
		placeCursor(c, r, value, input)
		return
	}
	lo, hi := -1, -1
	if input != nil && input.Visual {
		lo, hi = te.SelectionRange(input.Anchor, input.Cursor)
	}
	at := 0
	for i, line := range strings.Split(value, "\n") {
		if i < r.H {
			drawSelectable(c, r.Row(i), []rune(line), at, lo, hi)
		}
		at += len([]rune(line)) + 1
	}
	placeCursor(c, r, value, input)
}

func drawSelectable(c *Canvas, row Rect, line []rune, at, lo, hi int) {
	x := row.X
	for i, ch := range line {
		st := styleNone
		if at+i >= lo && at+i <= hi {
			st = styleSelection
		}
		x += c.Text(x, row.Y, row.X+row.W-x, string(ch), st)
	}
}

func placeCursor(c *Canvas, r Rect, value string, input *TextInput) {
	if input == nil {
		return
	}
	row, col := cursorCell(value, input.Cursor)
	c.SetCursor(r.X+min(col, max(r.W-1, 0)), r.Y+min(row, max(r.H-1, 0)))
}

func cursorCell(value string, cursor int) (int, int) {
	runes := []rune(value)
	before := string(runes[:min(cursor, len(runes))])
	row := strings.Count(before, "\n")
	return row, Width(before[strings.LastIndex(before, "\n")+1:])
}
