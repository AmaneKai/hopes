package ui

import (
	"strconv"
	"strings"

	"github.com/clipperhouse/displaywidth"
)

type Color uint16

const DefaultColor Color = 0

func Ansi(n uint8) Color { return Color(n) + 1 }

type Attr uint8

const (
	Bold Attr = 1 << iota
	Dim
	Underline
	Reverse
	Strike
)

type Style struct {
	Fg, Bg Color
	Attrs  Attr
}

func (s Style) Patch(o Style) Style {
	if o.Fg != DefaultColor {
		s.Fg = o.Fg
	}
	if o.Bg != DefaultColor {
		s.Bg = o.Bg
	}
	s.Attrs |= o.Attrs
	return s
}

func (s Style) With(a Attr) Style { return Style{Fg: s.Fg, Bg: s.Bg, Attrs: s.Attrs | a} }

type Span struct {
	Text  string
	Style Style
}

func Plain(text string) Span { return Span{Text: text} }

func Styled(text string, st Style) Span { return Span{Text: text, Style: st} }

type Rect struct{ X, Y, W, H int }

func (r Rect) Empty() bool { return r.W <= 0 || r.H <= 0 }

func (r Rect) Inset(left, top, right, bottom int) Rect {
	return Rect{r.X + left, r.Y + top, max(r.W-left-right, 0), max(r.H-top-bottom, 0)}
}

func (r Rect) Row(i int) Rect { return Rect{r.X, r.Y + i, r.W, 1} }

func (r Rect) Centered(w, h int) Rect {
	w, h = max(min(w, r.W-2), 1), max(min(h, r.H-2), 1)
	return Rect{r.X + (r.W-w)/2, r.Y + (r.H-h)/2, w, h}
}

const Fill = -1

func SplitV(r Rect, sizes ...int) []Rect {
	return split(r.H, sizes, func(at, n int) Rect { return Rect{r.X, r.Y + at, r.W, n} })
}

func SplitH(r Rect, gap int, sizes ...int) []Rect {
	spaced := make([]int, 0, len(sizes)*2)
	for i, s := range sizes {
		if i > 0 {
			spaced = append(spaced, gap)
		}
		spaced = append(spaced, s)
	}
	parts := split(r.W, spaced, func(at, n int) Rect { return Rect{r.X + at, r.Y, n, r.H} })
	out := make([]Rect, 0, len(sizes))
	for i := 0; i < len(parts); i += 2 {
		out = append(out, parts[i])
	}
	return out
}

func split(total int, sizes []int, build func(at, n int) Rect) []Rect {
	fixed, fills := 0, 0
	for _, s := range sizes {
		if s == Fill {
			fills++
		} else {
			fixed += s
		}
	}
	room, rects, at := max(total-fixed, 0), make([]Rect, len(sizes)), 0
	for i, s := range sizes {
		n := s
		if s == Fill {
			n = room / fills
			room -= n
			fills--
		}
		n = max(min(n, total-at), 0)
		rects[i] = build(at, n)
		at += n
	}
	return rects
}

type cell struct {
	text  string
	style Style
	cont  bool
}

type Canvas struct {
	W, H   int
	cells  []cell
	Cursor *[2]int
}

func NewCanvas(w, h int) *Canvas {
	c := &Canvas{W: max(w, 0), H: max(h, 0)}
	c.cells = make([]cell, c.W*c.H)
	c.Fill(Rect{0, 0, c.W, c.H}, Style{})
	return c
}

func (c *Canvas) at(x, y int) *cell {
	if x < 0 || y < 0 || x >= c.W || y >= c.H {
		return nil
	}
	return &c.cells[y*c.W+x]
}

func (c *Canvas) put(x, y int, text string, st Style, cont bool) {
	target := c.at(x, y)
	if target == nil {
		return
	}
	if target.cont {
		c.blank(x-1, y)
	}
	if next := c.at(x+1, y); next != nil && next.cont && !cont {
		next.cont, next.text = false, " "
	}
	*target = cell{text: text, style: st, cont: cont}
}

func (c *Canvas) blank(x, y int) {
	if lead := c.at(x, y); lead != nil {
		lead.text, lead.cont = " ", false
	}
}

func (c *Canvas) clip(r Rect) Rect {
	x0, y0 := max(r.X, 0), max(r.Y, 0)
	x1, y1 := min(r.X+r.W, c.W), min(r.Y+r.H, c.H)
	return Rect{x0, y0, max(x1-x0, 0), max(y1-y0, 0)}
}

func (c *Canvas) Fill(r Rect, st Style) {
	r = c.clip(r)
	for y := r.Y; y < r.Y+r.H; y++ {
		for x := r.X; x < r.X+r.W; x++ {
			c.put(x, y, " ", st, false)
		}
	}
}

func (c *Canvas) Paint(r Rect, st Style) {
	r = c.clip(r)
	for y := r.Y; y < r.Y+r.H; y++ {
		for x := r.X; x < r.X+r.W; x++ {
			cl := c.at(x, y)
			cl.style = cl.style.Patch(st)
		}
	}
}

func isPrintableASCII(text string) bool {
	for i := 0; i < len(text); i++ {
		if text[i] < 0x20 || text[i] > 0x7e {
			return false
		}
	}
	return true
}

func (c *Canvas) Text(x, y, width int, text string, st Style) int {
	if isPrintableASCII(text) {
		n := min(len(text), width)
		for i := 0; i < n; i++ {
			c.put(x+i, y, text[i:i+1], c.styleAt(x+i, y).Patch(st), false)
		}
		return max(n, 0)
	}
	used := 0
	gr := displaywidth.StringGraphemes(text)
	for gr.Next() && used < width {
		w := gr.Width()
		if used+w > width {
			c.put(x+used, y, " ", c.styleAt(x+used, y).Patch(st), false)
			return used + 1
		}
		c.putGrapheme(x+used, y, gr.Value(), w, st)
		used += w
	}
	return used
}

func (c *Canvas) putGrapheme(x, y int, text string, w int, st Style) {
	if text == "\t" || text == "\n" || text == "\r" {
		text = " "
	}
	c.put(x, y, text, c.styleAt(x, y).Patch(st), false)
	for i := 1; i < w; i++ {
		c.put(x+i, y, "", c.styleAt(x+i, y).Patch(st), true)
	}
}

func (c *Canvas) styleAt(x, y int) Style {
	if cl := c.at(x, y); cl != nil {
		return Style{Bg: cl.style.Bg}
	}
	return Style{}
}

func (c *Canvas) Line(r Rect, spans ...Span) int {
	used := 0
	for _, s := range spans {
		used += c.Text(r.X+used, r.Y, r.W-used, s.Text, s.Style)
	}
	return used
}

func (c *Canvas) Lines(r Rect, lines [][]Span) {
	for i, line := range lines {
		if i >= r.H {
			return
		}
		c.Line(r.Row(i), line...)
	}
}

func (c *Canvas) Rule(r Rect, st Style) {
	c.Text(r.X, r.Y, r.W, strings.Repeat("─", max(r.W, 0)), st)
}

func (c *Canvas) Box(r Rect, st Style) {
	if r.W < 2 || r.H < 2 {
		return
	}
	right, bottom := r.X+r.W-1, r.Y+r.H-1
	horizontal := strings.Repeat("─", r.W-2)
	c.Text(r.X, r.Y, r.W, "┌"+horizontal+"┐", st)
	c.Text(r.X, bottom, r.W, "└"+horizontal+"┘", st)
	for y := r.Y + 1; y < bottom; y++ {
		c.Text(r.X, y, 1, "│", st)
		c.Text(right, y, 1, "│", st)
	}
}

func (c *Canvas) SetCursor(x, y int) {
	if c.at(x, y) != nil {
		c.Cursor = &[2]int{x, y}
	}
}

func (c *Canvas) PlainText() string {
	var b strings.Builder
	for y := range c.H {
		for x := range c.W {
			b.WriteString(c.cells[y*c.W+x].text)
		}
		b.WriteByte('\n')
	}
	return b.String()
}

func (c *Canvas) StyleAt(x, y int) Style {
	if cl := c.at(x, y); cl != nil {
		return cl.style
	}
	return Style{}
}

func (c *Canvas) ANSI() string {
	var b strings.Builder
	b.Grow(c.W*c.H + c.H*16)
	for y := range c.H {
		if y > 0 {
			b.WriteByte('\n')
		}
		c.writeRow(&b, y)
	}
	return b.String()
}

func (c *Canvas) writeRow(b *strings.Builder, y int) {
	current := Style{}
	for x := range c.W {
		cl := c.cells[y*c.W+x]
		if cl.style != current {
			b.WriteString(sgr(cl.style))
			current = cl.style
		}
		b.WriteString(cl.text)
	}
	if current != (Style{}) {
		b.WriteString("\x1b[0m")
	}
}

var attrCodes = []struct {
	attr Attr
	code string
}{{Bold, "1"}, {Dim, "2"}, {Underline, "4"}, {Reverse, "7"}, {Strike, "9"}}

func sgr(st Style) string {
	buf := make([]byte, 0, 40)
	buf = append(buf, "\x1b[0"...)
	for _, a := range attrCodes {
		if st.Attrs&a.attr != 0 {
			buf = append(buf, ';')
			buf = append(buf, a.code...)
		}
	}
	buf = appendColor(buf, st.Fg, 30, 90, "38")
	buf = appendColor(buf, st.Bg, 40, 100, "48")
	return string(append(buf, 'm'))
}

func appendColor(buf []byte, c Color, base, bright int, extended string) []byte {
	if c == DefaultColor {
		return buf
	}
	n := int(c) - 1
	buf = append(buf, ';')
	switch {
	case n < 8:
		return strconv.AppendInt(buf, int64(base+n), 10)
	case n < 16:
		return strconv.AppendInt(buf, int64(bright+n-8), 10)
	}
	buf = append(buf, extended...)
	buf = append(buf, ";5;"...)
	return strconv.AppendInt(buf, int64(n), 10)
}

func Width(text string) int { return displaywidth.String(text) }

func Ellipsize(text string, width int) string {
	return displaywidth.TruncateString(text, max(width, 0), "\u2026")
}
