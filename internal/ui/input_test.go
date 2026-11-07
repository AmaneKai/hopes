package ui

import "testing"

func typeInto(in *TextInput, buf *string, text string) {
	for _, r := range text {
		in.HandleKey(buf, Rune(r), false)
	}
}

func normalAt(cursor int) TextInput { return TextInput{Cursor: cursor} }

func TestInsertModeTypesAtTheCursor(t *testing.T) {
	in, buf := NewTextInput(), ""
	typeInto(&in, &buf, "helloworld")
	expect(t, in.Cursor, 10)
	for range 5 {
		in.HandleKey(&buf, Special(KeyLeft), false)
	}
	in.HandleKey(&buf, Rune(' '), false)
	expect(t, buf, "hello world")
	expect(t, in.Cursor, 6)
}

func TestEscLeavesInsertBeforeReportingClose(t *testing.T) {
	in, buf := NewTextInput(), "draft"
	in.Focus(buf)
	expectTrue(t, in.HandleEsc(&buf), "first esc leaves insert")
	expectTrue(t, !in.Insert, "normal mode expected")
	expectTrue(t, !in.HandleEsc(&buf), "second esc reports close")
}

func TestDdInNormalModeClearsASingleLineField(t *testing.T) {
	in, buf := normalAt(3), "delete me"
	in.HandleKey(&buf, Rune('d'), false)
	expectTrue(t, in.PendingD, "d is pending")
	in.HandleKey(&buf, Rune('d'), false)
	expect(t, buf, "")
}

func TestFocusResetsToInsertAtTheEnd(t *testing.T) {
	in := normalAt(0)
	in.Focus("abc")
	expectTrue(t, in.Insert, "insert expected")
	expect(t, in.Cursor, 3)
}

func TestVisualDeleteYankAndChange(t *testing.T) {
	in, buf := normalAt(0), "hello world"
	typeInto(&in, &buf, "vlllld")
	expect(t, buf, " world")
	expect(t, in.Clipboard, "hello")

	in, buf = normalAt(6), "hello world"
	typeInto(&in, &buf, "vlllly")
	expect(t, in.Clipboard, "world")
	in.Cursor = 4
	in.HandleKey(&buf, Rune('p'), false)
	expect(t, buf, "helloworld world")

	in, buf = normalAt(1), "abcdef"
	typeInto(&in, &buf, "vlc")
	expect(t, buf, "adef")
	expectTrue(t, in.Insert && !in.Visual, "change enters insert")
}

func TestEscExitsVisualWithoutEditing(t *testing.T) {
	in, buf := normalAt(0), "keep me"
	in.HandleKey(&buf, Rune('v'), false)
	expectTrue(t, in.HandleEsc(&buf), "esc leaves visual")
	expect(t, buf, "keep me")
	expectTrue(t, !in.HandleEsc(&buf), "then reports close")
}

func TestPasteInsertsAtTheCursorAndFlattensSingleLineFields(t *testing.T) {
	in, buf := NewTextInput(), "ab"
	in.Focus(buf)
	in.HandleKey(&buf, Special(KeyLeft), false)
	in.Paste(&buf, "x\ny")
	expect(t, buf, "ax yb")
}
