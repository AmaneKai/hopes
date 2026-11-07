package ui

import (
	"unicode/utf8"

	tea "charm.land/bubbletea/v2"
)

type KeyCode int

const (
	KeyRune KeyCode = iota
	KeyEnter
	KeyEsc
	KeyTab
	KeyBackTab
	KeyBackspace
	KeyUp
	KeyDown
	KeyLeft
	KeyRight
	KeyHome
	KeyEnd
	KeyPageUp
	KeyPageDown
	KeyF5
	KeyUnknown
)

type Key struct {
	Code KeyCode
	Rune rune
	Ctrl bool
}

func Rune(r rune) Key       { return Key{Code: KeyRune, Rune: r} }
func CtrlRune(r rune) Key   { return Key{Code: KeyRune, Rune: r, Ctrl: true} }
func Special(c KeyCode) Key { return Key{Code: c} }

func (k Key) Is(r rune) bool     { return k.Code == KeyRune && k.Rune == r && !k.Ctrl }
func (k Key) IsCtrl(r rune) bool { return k.Code == KeyRune && k.Rune == r && k.Ctrl }

func (k Key) IsAny(runes ...rune) bool {
	for _, r := range runes {
		if k.Is(r) {
			return true
		}
	}
	return false
}

func (k Key) Digit() (rune, bool) {
	ok := k.Code == KeyRune && !k.Ctrl && k.Rune >= '0' && k.Rune <= '9'
	return k.Rune, ok
}

var specialKeys = map[rune]KeyCode{
	tea.KeyEnter:     KeyEnter,
	tea.KeyEscape:    KeyEsc,
	tea.KeyTab:       KeyTab,
	tea.KeyBackspace: KeyBackspace,
	tea.KeyUp:        KeyUp,
	tea.KeyDown:      KeyDown,
	tea.KeyLeft:      KeyLeft,
	tea.KeyRight:     KeyRight,
	tea.KeyHome:      KeyHome,
	tea.KeyEnd:       KeyEnd,
	tea.KeyPgUp:      KeyPageUp,
	tea.KeyPgDown:    KeyPageDown,
	tea.KeyF5:        KeyF5,
}

func KeyFromTea(msg tea.KeyPressMsg) Key {
	ctrl := msg.Mod.Contains(tea.ModCtrl)
	if msg.Code == tea.KeyTab && msg.Mod.Contains(tea.ModShift) {
		return Special(KeyBackTab)
	}
	if code, ok := specialKeys[msg.Code]; ok && !(ctrl && msg.Code == tea.KeyTab) {
		return Special(code)
	}
	if ctrl {
		return CtrlRune(msg.Code)
	}
	if r, _ := utf8.DecodeRuneInString(msg.Text); msg.Text != "" {
		return Rune(r)
	}
	return Special(KeyUnknown)
}
