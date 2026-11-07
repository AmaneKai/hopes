package textedit

import (
	"slices"
	"unicode"
)

func isWord(c rune) bool { return !unicode.IsSpace(c) }

func lineStartBefore(cs []rune, end int) int {
	for i := end - 1; i >= 0; i-- {
		if cs[i] == '\n' {
			return i + 1
		}
	}
	return 0
}

func lineEndFrom(cs []rune, from int) int {
	for i := from; i < len(cs); i++ {
		if cs[i] == '\n' {
			return i
		}
	}
	return len(cs)
}

func lineBounds(cs []rune, cursor int) (int, int) {
	cursor = min(cursor, len(cs))
	return lineStartBefore(cs, cursor), lineEndFrom(cs, cursor)
}

func lastPos(start, end int) int {
	if end > start {
		return end - 1
	}
	return start
}

func clampTo(cursor, n int) int { return max(0, min(cursor, n-1)) }

func Len(s string) int { return len([]rune(s)) }

func atCursor(s string, cursor int, f func(cs []rune, cursor int) int) int {
	cs := []rune(s)
	if len(cs) == 0 {
		return 0
	}
	return f(cs, clampTo(cursor, len(cs)))
}

func ClampNormal(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, end := lineBounds(cs, c)
		return max(start, min(c, lastPos(start, end)))
	})
}

func MoveLeft(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, _ := lineBounds(cs, c)
		return max(c-1, start)
	})
}

func MoveRight(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, end := lineBounds(cs, c)
		return min(c+1, lastPos(start, end))
	})
}

func MoveUp(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, _ := lineBounds(cs, c)
		if start == 0 {
			return c
		}
		prevEnd := start - 1
		prevStart := lineStartBefore(cs, prevEnd)
		return min(prevStart+c-start, lastPos(prevStart, prevEnd))
	})
}

func MoveDown(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, end := lineBounds(cs, c)
		if end >= len(cs) {
			return c
		}
		nextStart := end + 1
		nextEnd := lineEndFrom(cs, nextStart)
		return min(nextStart+c-start, lastPos(nextStart, nextEnd))
	})
}

func CaretLeft(s string, cursor int) int {
	cs := []rune(s)
	cursor = min(cursor, len(cs))
	start, _ := lineBounds(cs, cursor)
	return max(cursor-1, start)
}

func CaretRight(s string, cursor int) int {
	cs := []rune(s)
	cursor = min(cursor, len(cs))
	_, end := lineBounds(cs, cursor)
	return min(cursor+1, end)
}

func CaretUp(s string, cursor int) int {
	cs := []rune(s)
	cursor = min(cursor, len(cs))
	start, _ := lineBounds(cs, cursor)
	if start == 0 {
		return cursor
	}
	col := cursor - start
	prevEnd := start - 1
	prevStart := lineStartBefore(cs, prevEnd)
	return min(prevStart+col, prevEnd)
}

func CaretDown(s string, cursor int) int {
	cs := []rune(s)
	n := len(cs)
	cursor = min(cursor, n)
	start, end := lineBounds(cs, cursor)
	if end >= n {
		return cursor
	}
	col := cursor - start
	nextStart := end + 1
	return min(nextStart+col, lineEndFrom(cs, nextStart))
}

func LineStart(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, _ := lineBounds(cs, c)
		return start
	})
}

func LineEnd(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, end := lineBounds(cs, c)
		return lastPos(start, end)
	})
}

func WordForward(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, i int) int {
		n := len(cs)
		for i < n && isWord(cs[i]) {
			i++
		}
		for i < n && !isWord(cs[i]) {
			i++
		}
		return min(i, n-1)
	})
}

func WordEnd(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		n, i := len(cs), c+1
		for i < n && !isWord(cs[i]) {
			i++
		}
		for i+1 < n && isWord(cs[i+1]) {
			i++
		}
		return min(i, n-1)
	})
}

func WordBack(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, i int) int {
		if i == 0 {
			return 0
		}
		i--
		for i > 0 && !isWord(cs[i]) {
			i--
		}
		for i > 0 && isWord(cs[i-1]) {
			i--
		}
		return i
	})
}

func InsertText(s *string, cursor int, text string) int {
	cs := []rune(*s)
	at := max(0, min(cursor, len(cs)))
	ins := []rune(text)
	*s = string(slices.Insert(cs, at, ins...))
	return at + len(ins)
}

func InsertChar(s *string, cursor int, c rune) int { return InsertText(s, cursor, string(c)) }

func Backspace(s *string, cursor int) int {
	cs := []rune(*s)
	at := min(cursor, len(cs))
	if at <= 0 {
		return 0
	}
	*s = string(slices.Delete(cs, at-1, at))
	return at - 1
}

func DeleteChar(s *string, cursor int) int {
	cs := []rune(*s)
	if len(cs) == 0 {
		return 0
	}
	cursor = clampTo(cursor, len(cs))
	start, end := lineBounds(cs, cursor)
	if end > start {
		*s = string(slices.Delete(cs, cursor, cursor+1))
	}
	return ClampNormal(*s, cursor)
}

func DeleteToLineEnd(s *string, cursor int) int {
	cs := []rune(*s)
	cursor = min(cursor, len(cs))
	start, end := lineBounds(cs, cursor)
	if end > cursor {
		*s = string(slices.Delete(cs, cursor, end))
	}
	return ClampNormal(*s, min(max(cursor, start), Len(*s)))
}

func DeleteLine(s *string, cursor int) int {
	cs := []rune(*s)
	n := len(cs)
	if n == 0 {
		return 0
	}
	start, end := lineBounds(cs, clampTo(cursor, n))
	from, to := start, end
	if end < n {
		to = end + 1
	} else if start > 0 {
		from = start - 1
	}
	*s = string(slices.Delete(cs, from, to))
	return ClampNormal(*s, from)
}

func EnterInsertBefore(s string, cursor int) int { return ClampNormal(s, cursor) }

func EnterInsertAfter(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		start, end := lineBounds(cs, c)
		return max(min(c+1, end), start)
	})
}

func EnterInsertLineStart(s string, cursor int) int { return LineStart(s, cursor) }

func EnterInsertLineEnd(s string, cursor int) int {
	return atCursor(s, cursor, func(cs []rune, c int) int {
		_, end := lineBounds(cs, c)
		return end
	})
}

func OpenBelow(s *string, cursor int, allowNewline bool) int {
	if !allowNewline {
		return EnterInsertLineEnd(*s, cursor)
	}
	cs := []rune(*s)
	_, end := lineBounds(cs, clampTo(cursor, len(cs)))
	*s = string(slices.Insert(cs, end, '\n'))
	return end + 1
}

func OpenAbove(s *string, cursor int, allowNewline bool) int {
	if !allowNewline {
		return EnterInsertLineStart(*s, cursor)
	}
	cs := []rune(*s)
	start, _ := lineBounds(cs, clampTo(cursor, len(cs)))
	*s = string(slices.Insert(cs, start, '\n'))
	return start
}

func SelectionRange(anchor, cursor int) (int, int) {
	return min(anchor, cursor), max(anchor, cursor)
}

func YankRange(s string, lo, hi int) string {
	cs := []rune(s)
	n := len(cs)
	if n == 0 || lo >= n {
		return ""
	}
	return string(cs[lo : min(hi, n-1)+1])
}

func DeleteRange(s *string, lo, hi int) int {
	cs := []rune(*s)
	n := len(cs)
	if n == 0 || lo >= n {
		return ClampNormal(*s, lo)
	}
	hi = min(hi, n-1)
	*s = string(slices.Delete(cs, lo, hi+1))
	return ClampNormal(*s, lo)
}

func PasteAfter(s *string, cursor int, text string) int {
	if text == "" {
		return cursor
	}
	at := min(cursor+1, Len(*s))
	InsertText(s, at, text)
	return ClampNormal(*s, at+max(Len(text)-1, 0))
}

func PasteBefore(s *string, cursor int, text string) int {
	if text == "" {
		return cursor
	}
	at := min(cursor, Len(*s))
	InsertText(s, at, text)
	return ClampNormal(*s, at+max(Len(text)-1, 0))
}
