package ui

import (
	"errors"
	"os"
	"strings"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
)

func lastLine(screen string) string {
	lines := strings.Split(strings.TrimRight(screen, "\n"), "\n")
	return lines[len(lines)-1]
}

func TestTheMainScreenShowsTasksStatusAndHints(t *testing.T) {
	screen := newSession(t, seed()).screen(120, 32)
	parts := []string{
		"Write compiler backend", "NORMAL", "1 / 3", "help", "Workspaces", "Hopes / Work",
	}
	for _, part := range parts {
		expectContains(t, screen, part)
	}
	expectTrue(t, !strings.Contains(screen, "Inspector"), "no inspector expected")
}

func TestTheFindPromptShowsWhatYouTypeOnTheBottomRow(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('/'))
	s.typeStr("tui")
	screen := s.screen(120, 32)
	expectTrue(t, strings.HasPrefix(lastLine(screen), "/tui"), lastLine(screen))
	expectContains(t, screen, "FIND")
}

func TestTheCommandLineShowsOnTheBottomRow(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune(':'))
	s.typeStr("sy")
	screen := s.screen(120, 32)
	expectTrue(t, strings.HasPrefix(lastLine(screen), ":sy"), lastLine(screen))
	expectContains(t, screen, "COMMAND")
}

func TestTheFormAndHelpPopupsRender(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('a'))
	expectContains(t, s.screen(120, 32), "New task")
	s.press(esc, esc, Rune('?'))
	expectContains(t, s.screen(120, 32), "Keyboard shortcuts")
}

func TestPopupsHaveSquareCorners(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('?'))
	screen := s.screen(120, 32)
	expectContains(t, screen, "┌")
	expectTrue(t, !strings.ContainsAny(screen, "╭╮╯╰"), "rounded corner found")
}

func TestTheBoardRendersThreeColumns(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'))
	screen := s.screen(120, 32)
	for _, part := range []string{"To Do", "In Progress", "Completed"} {
		expectContains(t, screen, part)
	}
}

func TestEveryModeRendersAtEverySize(t *testing.T) {
	sizes := [][2]int{{120, 32}, {90, 24}, {60, 20}, {30, 8}, {12, 5}, {3, 2}, {1, 1}, {0, 0}}
	setups := map[string][]Key{
		"table": nil, "board": {Rune('v')}, "new task": {Rune('a')}, "edit task": {Rune('e')},
		"find": {Rune('/')}, "command": {Rune(':')}, "help": {Rune('?')}, "sidebar": {Rune('h')},
	}
	for name, keys := range setups {
		for _, size := range sizes {
			s := newSession(t, seed())
			s.press(keys...)
			t.Run(name, func(t *testing.T) { s.screen(size[0], size[1]) })
		}
	}
}

func TestAnEmptyWorkspaceRendersAndAcceptsInput(t *testing.T) {
	s := newSession(t, nil)
	expect(t, s.cursor(), -1)
	expectContains(t, s.screen(120, 32), "add a task to Inbox")
	s.press(Rune('j'), Rune('G'), Rune(' '), Rune('e'), Rune('p'))
	expect(t, len(s.items()), 0)
	expect(t, s.app.Mode(), ModeNormal)
}

func TestTheBoardSurvivesWideCharactersInNotes(t *testing.T) {
	items := seed()
	items[0].Description = strings.Repeat("雨音カイ", 20) + strings.Repeat("\U0001F600", 20)
	items[1].Description = strings.Repeat("é", 40)
	s := newSession(t, items)
	s.press(Rune('v'))
	expectContains(t, s.screen(120, 30), "...")
}

func TestTheSelectedRowIsHighlightedAcrossTheWholeRow(t *testing.T) {
	s := newSession(t, seed())
	c := s.app.Render(100, 16)
	row := strings.Index(strings.Split(c.PlainText(), "\n")[7], "Write")
	expectTrue(t, row > 0, "first task row expected on line 7")
	expect(t, c.StyleAt(row, 7).Bg, styleRowActive.Bg)
	expect(t, c.StyleAt(c.W-3, 7).Bg, styleRowActive.Bg)
}

func TestACorruptItemsFileIsNeverOverwritten(t *testing.T) {
	dir := t.TempDir()
	path := writeFile(t, dir, "items.json", "[{ this is not json")
	s := &session{t: t, app: openApp(dir), dir: dir}
	expectContains(t, s.status(), "saving is disabled")
	s.press(Rune('a'))
	s.typeStr("new")
	s.press(enter)
	data, err := os.ReadFile(path)
	must(t, err)
	expect(t, string(data), "[{ this is not json")
	expectContains(t, s.status(), "not saved")
}

type failingStore struct{ dir string }

func (f failingStore) Load() ([]domain.Item, error) {
	return []domain.Item{domain.NewItem("kept", "Inbox")}, nil
}
func (f failingStore) Save([]domain.Item) error { return errors.New("disk full") }
func (f failingStore) DataDir() string          { return f.dir }

func TestAFailedSaveIsReportedInsteadOfClaimingSuccess(t *testing.T) {
	dir := t.TempDir()
	s := &session{t: t, app: New(failingStore{dir}, idleSync(dir)), dir: dir}
	s.press(Rune('a'))
	s.typeStr("new")
	s.press(enter)
	expectContains(t, s.status(), "disk full")
	expectTrue(t, !strings.Contains(s.status(), "Task created"), s.status())
}
