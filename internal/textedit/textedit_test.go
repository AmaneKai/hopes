package textedit

import "testing"

func eq[T comparable](t *testing.T, got, want T) {
	t.Helper()
	if got != want {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestHLStayWithinLine(t *testing.T) {
	s := "abc\ndef"
	eq(t, MoveLeft(s, 0), 0)
	eq(t, MoveRight(s, 2), 2)
	eq(t, MoveRight(s, 3), 2)
}

func TestCaretLeftRightRestPastLastChar(t *testing.T) {
	s := "helloworld"
	eq(t, CaretLeft(s, 10), 9)
	eq(t, CaretRight(s, 10), 10)
	eq(t, CaretLeft(s, 0), 0)
	c := 10
	for range 5 {
		c = CaretLeft(s, c)
	}
	eq(t, c, 5)
}

func TestCaretUpDownRestPastLastCharOfEachLine(t *testing.T) {
	eq(t, CaretDown("ab\ncd", 2), 5)
	eq(t, CaretUp("ab\ncd", 5), 2)
}

func TestZeroAndDollar(t *testing.T) {
	s := "hello world\nsecond"
	eq(t, LineStart(s, 8), 0)
	eq(t, LineEnd(s, 2), 10)
	eq(t, LineStart(s, 14), 12)
	eq(t, LineEnd(s, 14), 17)
}

func TestWordMotions(t *testing.T) {
	s := "foo   bar baz"
	eq(t, WordForward(s, 0), 6)
	eq(t, WordForward(s, 6), 10)
	eq(t, WordEnd(s, 0), 2)
	eq(t, WordEnd(s, 2), 8)
	eq(t, WordBack(s, 10), 6)
	eq(t, WordBack(s, 6), 0)
}

func TestUpDownClampIntoShortLines(t *testing.T) {
	s := "abcdef\nxy\nqrstuv"
	down := MoveDown(s, 4)
	eq(t, down, 8)
	down2 := MoveDown(s, down)
	eq(t, down2, 11)
	eq(t, MoveUp(s, down2), 8)
}

func TestEdits(t *testing.T) {
	s := "abc"
	eq(t, DeleteChar(&s, 2), 1)
	eq(t, s, "ab")

	s = "one\ntwo\nthree"
	eq(t, DeleteLine(&s, 5), 4)
	eq(t, s, "one\nthree")

	s = "solo"
	eq(t, DeleteLine(&s, 2), 0)
	eq(t, s, "")

	s = "keep this\nand this"
	eq(t, DeleteToLineEnd(&s, 4), 3)
	eq(t, s, "keep\nand this")

	s = "helloworld"
	c := InsertChar(&s, 5, ' ')
	eq(t, s, "hello world")
	eq(t, c, 6)
	eq(t, Backspace(&s, c), 5)
	eq(t, s, "helloworld")
}

func TestOpenLines(t *testing.T) {
	s := "title"
	eq(t, OpenBelow(&s, 0, false), 5)
	eq(t, s, "title")
	s = "one"
	eq(t, OpenBelow(&s, 1, true), 4)
	eq(t, s, "one\n")
}

func TestInsertPositions(t *testing.T) {
	s := "hey\nyou"
	eq(t, EnterInsertBefore(s, 5), 5)
	eq(t, EnterInsertAfter(s, 1), 2)
	eq(t, EnterInsertLineStart(s, 5), 4)
	eq(t, EnterInsertLineEnd(s, 5), 7)
}

func TestEmptyBufferIsSafe(t *testing.T) {
	eq(t, ClampNormal("", 0), 0)
	eq(t, MoveLeft("", 0), 0)
	eq(t, MoveRight("", 0), 0)
	eq(t, WordForward("", 0), 0)
	eq(t, LineEnd("", 0), 0)
	s := ""
	eq(t, DeleteChar(&s, 0), 0)
	eq(t, DeleteLine(&s, 0), 0)
}

func TestVisualSelectionYankDeletePaste(t *testing.T) {
	lo, hi := SelectionRange(6, 2)
	eq(t, lo, 2)
	eq(t, hi, 6)
	eq(t, YankRange("hello world", 0, 4), "hello")

	s := "hello world"
	eq(t, DeleteRange(&s, 0, 4), 0)
	eq(t, s, " world")

	s = "hello world"
	clip := YankRange(s, 6, 10)
	eq(t, PasteAfter(&s, 4, clip), 9)
	eq(t, s, "helloworld world")

	s = "hi"
	eq(t, PasteBefore(&s, 0, "XY"), 1)
	eq(t, s, "XYhi")

	s = "abc"
	eq(t, PasteAfter(&s, 1, ""), 1)
	eq(t, s, "abc")
}
