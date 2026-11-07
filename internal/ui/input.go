package ui

import (
	"strings"

	te "github.com/AmaneKai/hopes/internal/textedit"
)

type TextInput struct {
	Cursor     int
	Insert     bool
	PendingD   bool
	Visual     bool
	Anchor     int
	Clipboard  string
	allowLines bool
}

type motion func(string, int) int

var normalMotions = map[rune]motion{
	'h': te.MoveLeft, '0': te.LineStart, '$': te.LineEnd,
	'l': te.MoveRight, 'w': te.WordForward, 'e': te.WordEnd, 'b': te.WordBack,
}

var arrowMotions = map[KeyCode]motion{KeyLeft: te.MoveLeft, KeyRight: te.MoveRight}

var lineMotions = map[KeyCode]motion{KeyUp: te.MoveUp, KeyDown: te.MoveDown}

var lineRuneMotions = map[rune]motion{'k': te.MoveUp, 'j': te.MoveDown}

var insertEntries = map[rune]motion{
	'i': te.EnterInsertBefore, 'a': te.EnterInsertAfter,
	'I': te.EnterInsertLineStart, 'A': te.EnterInsertLineEnd,
}

func NewTextInput() TextInput { return TextInput{Insert: true} }

func (t *TextInput) Focus(buf string) {
	t.Insert, t.PendingD, t.Visual = true, false, false
	t.Cursor = te.Len(buf)
}

func (t *TextInput) HandleEsc(buf *string) bool {
	if t.Insert {
		t.Insert, t.PendingD = false, false
		t.Cursor = te.ClampNormal(*buf, t.Cursor)
		return true
	}
	wasVisual := t.Visual
	t.Visual = false
	return wasVisual
}

func (t *TextInput) HandleKey(buf *string, k Key, allowNewline bool) {
	t.allowLines = allowNewline
	switch {
	case t.Insert:
		t.insertKey(buf, k)
	case t.PendingD && k.Is('d'):
		t.PendingD = false
		t.Cursor = te.DeleteLine(buf, t.Cursor)
	case t.Visual:
		t.PendingD = false
		t.visualKey(buf, k)
	default:
		t.PendingD = false
		t.normalKey(buf, k)
	}
}

func (t *TextInput) Paste(buf *string, text string) {
	if !t.allowLines {
		text = strings.ReplaceAll(strings.ReplaceAll(text, "\r\n", " "), "\n", " ")
	}
	if t.Insert {
		t.Cursor = te.InsertText(buf, t.Cursor, text)
	}
}

func (t *TextInput) motionFor(k Key) motion {
	if m, ok := arrowMotions[k.Code]; ok {
		return m
	}
	if m, ok := lineMotions[k.Code]; ok && t.allowLines {
		return m
	}
	if k.Code != KeyRune || k.Ctrl {
		return nil
	}
	if m, ok := lineRuneMotions[k.Rune]; ok && t.allowLines {
		return m
	}
	return normalMotions[k.Rune]
}

func (t *TextInput) normalKey(buf *string, k Key) {
	if m := t.motionFor(k); m != nil {
		t.Cursor = m(*buf, t.Cursor)
		return
	}
	if m, ok := insertEntries[k.Rune]; ok && k.Code == KeyRune && !k.Ctrl {
		t.Cursor, t.Insert = m(*buf, t.Cursor), true
		return
	}
	t.normalEdit(buf, k)
}

func (t *TextInput) normalEdit(buf *string, k Key) {
	switch {
	case k.Is('d'):
		t.PendingD = true
	case k.Is('v'):
		t.Visual, t.Anchor = true, t.Cursor
	case k.Is('p'):
		t.Cursor = te.PasteAfter(buf, t.Cursor, t.Clipboard)
	case k.Is('P'):
		t.Cursor = te.PasteBefore(buf, t.Cursor, t.Clipboard)
	case k.Is('x'):
		t.Cursor = te.DeleteChar(buf, t.Cursor)
	case k.Is('D'):
		t.Cursor = te.DeleteToLineEnd(buf, t.Cursor)
	case k.Is('o'):
		t.Cursor, t.Insert = te.OpenBelow(buf, t.Cursor, t.allowLines), true
	case k.Is('O'):
		t.Cursor, t.Insert = te.OpenAbove(buf, t.Cursor, t.allowLines), true
	}
}

func (t *TextInput) insertKey(buf *string, k Key) {
	switch {
	case k.IsCtrl('w'):
		*buf = deleteWordBack(*buf)
		t.Cursor = te.Len(*buf)
	case k.IsCtrl('u'):
		*buf, t.Cursor = "", 0
	case k.Code == KeyEnter && t.allowLines:
		t.Cursor = te.InsertChar(buf, t.Cursor, '\n')
	case k.Code == KeyRune && !k.Ctrl:
		t.Cursor = te.InsertChar(buf, t.Cursor, k.Rune)
	case k.Code == KeyBackspace:
		t.Cursor = te.Backspace(buf, t.Cursor)
	default:
		t.caretKey(*buf, k)
	}
}

func (t *TextInput) caretKey(buf string, k Key) {
	carets := map[KeyCode]motion{KeyLeft: te.CaretLeft, KeyRight: te.CaretRight}
	if t.allowLines {
		carets[KeyUp], carets[KeyDown] = te.CaretUp, te.CaretDown
	}
	if m, ok := carets[k.Code]; ok {
		t.Cursor = m(buf, t.Cursor)
	}
}

func (t *TextInput) visualKey(buf *string, k Key) {
	if m := t.motionFor(k); m != nil {
		t.Cursor = m(*buf, t.Cursor)
		return
	}
	lo, hi := te.SelectionRange(t.Anchor, t.Cursor)
	switch {
	case k.Is('v'):
		t.Visual = false
	case k.Is('y'):
		t.Clipboard = te.YankRange(*buf, lo, hi)
		t.Cursor, t.Visual = lo, false
	case k.IsAny('d', 'x', 'c'):
		t.Clipboard = te.YankRange(*buf, lo, hi)
		t.Cursor = te.DeleteRange(buf, lo, hi)
		t.Visual, t.Insert = false, k.Is('c')
	}
}

func deleteWordBack(s string) string {
	s = strings.TrimRight(s, " ")
	if i := strings.LastIndex(s, " "); i >= 0 {
		return s[:i+1]
	}
	return ""
}
