package ui

import (
	"slices"
	"testing"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/storage"
)

var (
	enter = Special(KeyEnter)
	esc   = Special(KeyEsc)
	tab   = Special(KeyTab)
)

func TestVimMotionsMoveTheCursor(t *testing.T) {
	s := newSession(t, seed())
	expect(t, s.cursor(), 0)
	s.press(Rune('j'))
	expect(t, s.cursor(), 1)
	s.press(Rune('G'))
	expect(t, s.cursor(), 2)
	s.typeStr("gg")
	expect(t, s.cursor(), 0)
}

func TestTheCursorStopsAtEitherEnd(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('k'))
	expect(t, s.cursor(), 0)
	s.press(Rune('G'), Rune('j'))
	expect(t, s.cursor(), 2)
}

func TestOnlyTheCurrentWorkspaceIsListed(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	expect(t, s.app.State.Workspace, "Work")
	expectVisible(t, s, 0, 2)
}

func TestTabCyclesWorkspacesAndWraps(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.press(tab)
	expect(t, s.app.State.Workspace, "Home")
	expectVisible(t, s, 1, 3)
	expect(t, s.cursor(), 1)
	s.press(tab)
	expect(t, s.app.State.Workspace, "Work")
	s.press(Special(KeyBackTab))
	expect(t, s.app.State.Workspace, "Home")
}

func TestCtrlWMovesFocusBetweenSidebarAndList(t *testing.T) {
	s := newSession(t, seed())
	expect(t, s.app.State.Focus, domain.FocusMain)
	s.press(CtrlRune('w'))
	expect(t, s.app.State.Focus, domain.FocusNav)
	s.press(CtrlRune('w'))
	expect(t, s.app.State.Focus, domain.FocusMain)
}

func TestTheSidebarSwitchesWorkspacesAsItsCursorMoves(t *testing.T) {
	s := newSession(t, append(twoWorkspaces(), domain.NewItem("g1", "Garden")))
	s.press(Rune('h'))
	expect(t, s.app.State.Focus, domain.FocusNav)
	s.press(Rune('j'))
	expect(t, s.app.State.Workspace, "Home")
	s.press(Rune('j'), Rune('j'))
	expect(t, s.app.State.Workspace, "Garden")
	s.press(Rune('1'))
	expect(t, s.app.State.Workspace, "Work")
	s.press(enter)
	expect(t, s.app.State.Focus, domain.FocusMain)
}

func TestWsCommandOpensOrCreatesAWorkspace(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.command("ws home")
	expect(t, s.app.State.Workspace, "Home")
	s.command("ws Garden")
	expect(t, s.app.State.Workspace, "Garden")
	expectVisible(t, s)
	expectTrue(t, hasWorkspaceNamed(s, "Garden"), "Garden should be listed")
	s.press(Rune('a'))
	s.typeStr("plant")
	s.press(enter)
	saved := s.saved()
	expect(t, saved[len(saved)-1].Workspace, "Garden")
}

func TestAnEmptyWorkspaceIsForgottenOnceYouLeaveIt(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.command("ws Scratch")
	s.press(tab)
	expectTrue(t, !hasWorkspaceNamed(s, "Scratch"), "Scratch should be gone")
}

func TestRenameMovesEveryTaskInTheWorkspace(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.command("rename Job")
	expect(t, s.app.State.Workspace, "Job")
	saved := s.saved()
	expect(t, saved[0].Workspace+saved[1].Workspace+saved[2].Workspace, "JobHomeJob")
	s.command("rename home")
	expect(t, s.app.State.Workspace, "Job")
	expectContains(t, s.status(), "already exists")
}

func TestEditingTheWorkspaceFieldMovesATaskAway(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.press(Rune('e'), tab, tab, tab, CtrlRune('u'))
	s.typeStr("Home")
	s.press(esc, CtrlRune('s'))
	expect(t, s.items()[0].Workspace, "Home")
	expect(t, s.app.State.Workspace, "Work")
	expectVisible(t, s, 2)
	expect(t, s.cursor(), 2)
}

func TestReorderingSkipsTasksInOtherWorkspaces(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.press(Rune('J'))
	expect(t, titles(s.items()), "w2,h1,w1,h2")
	expect(t, s.cursor(), 2)
}

func TestSearchStaysInsideTheCurrentWorkspace(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.app.setFilter("1")
	expectVisible(t, s, 0)
	s.press(tab)
	expectTrue(t, !s.app.State.HasFilter, "switching should clear the search")
}

func TestUndoingADeleteReturnsToItsWorkspace(t *testing.T) {
	s := newSession(t, twoWorkspaces())
	s.press(Rune('x'), tab)
	expect(t, s.app.State.Workspace, "Home")
	s.press(Rune('u'))
	expect(t, s.app.State.Workspace, "Work")
	expect(t, s.items()[0].Title, "w1")
}

func TestTasksSavedBeforeWorkspacesLandInTheirFirstTag(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "items.json", `[
		{"title":"a","priority":"Low","status":"Todo","tags":["uni","x"],"due":"2026-10-25"},
		{"title":"b","priority":"Low","status":"Todo"}
	]`)
	app := openApp(dir)
	expect(t, app.State.Items[0].Workspace, "uni")
	expect(t, app.State.Items[1].Workspace, "Inbox")
}

func TestAStrayKeyAbandonsAHalfTypedChord(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('d'))
	expect(t, s.app.PendingChord(), "d...")
	s.press(Rune('j'))
	expect(t, s.app.PendingChord(), "")
	s.press(Rune('d'))
	expect(t, len(s.items()), 3)
}

func TestSpaceCyclesStatus(t *testing.T) {
	s := newSession(t, seed())
	for _, want := range []domain.Status{domain.InProgress, domain.Complete, domain.Todo} {
		s.press(Rune(' '))
		expect(t, s.items()[0].Status, want)
	}
}

func TestPriorityKeysCycleAndSet(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('p'))
	expect(t, s.items()[0].Priority, domain.High)
	s.press(Rune('P'))
	expect(t, s.items()[0].Priority, domain.Urgent)
	s.press(Rune('4'))
	expect(t, s.items()[0].Priority, domain.Low)
}

func TestShiftJAndKReorderAndTheCursorFollows(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('J'))
	expect(t, s.items()[0].Title, "Design responsive TUI")
	expect(t, s.cursor(), 1)
	s.press(Rune('K'))
	expect(t, s.items()[0].Title, "Write compiler backend")
	expect(t, s.cursor(), 0)
}

func TestDdDeletesAndURestoresInPlace(t *testing.T) {
	s := newSession(t, seed())
	s.typeStr("dd")
	expect(t, len(s.items()), 2)
	expect(t, s.items()[0].Title, "Design responsive TUI")
	s.press(Rune('u'))
	expect(t, len(s.items()), 3)
	expect(t, s.items()[0].Title, "Write compiler backend")
	expect(t, s.cursor(), 0)
}

func TestDeletingTheLastRowKeepsTheCursorOnATask(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('G'), Rune('x'))
	expect(t, len(s.items()), 2)
	expect(t, s.cursor(), 1)
}

func TestDeletingEverythingLeavesNoSelection(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('x'), Rune('x'), Rune('x'), Rune('x'))
	expect(t, len(s.items()), 0)
	expect(t, s.cursor(), -1)
}

func TestCreatingATaskAddsSelectsAndSavesIt(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('a'))
	expect(t, s.app.Mode(), ModeCreating)
	s.typeStr("fresh")
	s.press(enter)
	expect(t, s.app.Mode(), ModeNormal)
	expect(t, len(s.items()), 4)
	expect(t, s.cursor(), 3)
	expect(t, s.saved()[3].Title, "fresh")
}

func TestCtrlSSavesFromAnyField(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('a'))
	s.typeStr("quick")
	s.press(tab, tab, Rune('1'), CtrlRune('s'))
	created := s.items()[3]
	expect(t, created.Title, "quick")
	expect(t, created.Priority, domain.Urgent)
	expect(t, created.Workspace, "Work")
}

func TestAnEmptyTitleKeepsTheFormOpen(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('a'), enter)
	expect(t, s.app.Mode(), ModeCreating)
	expect(t, len(s.items()), 3)
}

func TestEditingLoadsAndSavesTheSelectedTask(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('e'))
	expect(t, s.app.Mode(), ModeEditing)
	s.typeStr("!")
	s.press(enter)
	expect(t, s.app.Mode(), ModeNormal)
	expect(t, s.items()[0].Title, "Write compiler backend!")
}

func TestEscapingTheFormTwiceCancelsWithoutSaving(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('a'))
	s.typeStr("draft")
	s.press(esc)
	expect(t, s.app.Mode(), ModeCreating)
	s.press(esc)
	expect(t, s.app.Mode(), ModeNormal)
	expect(t, len(s.items()), 3)
}

func TestFindFiltersAndPutsTheCursorOnTheMatch(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('/'))
	expect(t, s.app.Mode(), ModeFinding)
	s.typeStr("responsive")
	s.press(enter)
	expect(t, s.app.Mode(), ModeNormal)
	expect(t, s.app.State.Filter, "responsive")
	expectVisible(t, s, 1)
	expect(t, s.cursor(), 1)
}

func TestEscapeCancelsFindWithoutFiltering(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('/'))
	s.typeStr("zzz")
	s.press(esc, esc)
	expect(t, s.app.Mode(), ModeNormal)
	expectTrue(t, !s.app.State.HasFilter, "no filter expected")
	expectVisible(t, s, 0, 1, 2)
}

func TestEscapeInNormalModeClearsTheFilter(t *testing.T) {
	s := newSession(t, seed())
	s.app.setFilter("compiler")
	expectVisible(t, s, 0)
	s.press(esc)
	expectTrue(t, !s.app.State.HasFilter, "filter should clear")
	expectVisible(t, s, 0, 1, 2)
}

func TestFiltersAndStatsReflectTheWorkspace(t *testing.T) {
	s := newSession(t, seed())
	expect(t, s.app.State.Stats().Total, 3)
	expect(t, s.app.State.Stats().Todo, 3)
	s.press(Rune(' '), Rune(' '))
	expect(t, s.app.State.Stats().Percent, 33)
	s.app.setFilter("responsive")
	expectVisible(t, s, 1)
}

func TestTaskCommandsWorkWhileTheSidebarHasFocus(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('h'), Rune('e'))
	expect(t, s.app.Mode(), ModeEditing)
}

func TestVTogglesBetweenTableAndBoard(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'))
	expect(t, s.app.State.ViewMode, domain.Kanban)
	s.press(Rune('v'))
	expect(t, s.app.State.ViewMode, domain.Table)
}

func TestShiftingACardMovesItAndTheCursorFollows(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'), Rune('L'))
	expect(t, s.items()[0].Status, domain.InProgress)
	expect(t, s.cursor(), 0)
	s.press(Rune('H'))
	expect(t, s.items()[0].Status, domain.Todo)
}

func TestHAndLSwitchBoardColumns(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'), Rune('l'))
	expect(t, s.cursor(), -1)
	s.press(Rune('h'))
	expect(t, s.cursor(), 0)
}

func TestTasksAddedFromTheBoardStartInThatColumn(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'), Rune('l'), Rune('a'))
	s.typeStr("board task")
	s.press(CtrlRune('s'))
	expect(t, s.app.Mode(), ModeNormal)
	expect(t, s.items()[3].Status, domain.InProgress)
}

func TestTablePagingKeysDoNothingOnTheBoard(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('v'))
	before := s.cursor()
	s.press(CtrlRune('d'), Special(KeyPageDown))
	expect(t, s.cursor(), before)
	expect(t, len(s.items()), 3)
}

func TestQuitting(t *testing.T) {
	s := newSession(t, seed())
	s.command("q")
	expectTrue(t, s.app.ShouldQuit(), ":q should quit")
	s = newSession(t, seed())
	s.press(Rune('q'))
	expectTrue(t, s.app.ShouldQuit(), "q should quit")
}

func TestCommandsReportAndReturnToNormal(t *testing.T) {
	s := newSession(t, seed())
	s.command("bogus")
	expect(t, s.app.Mode(), ModeNormal)
	expectContains(t, s.status(), "Unknown command")
	s.command("sync")
	expectContains(t, s.status(), "disabled")
}

func TestHelpOpensFromACommandAndClosesWithQ(t *testing.T) {
	s := newSession(t, seed())
	s.command("help")
	expect(t, s.app.Mode(), ModeHelp)
	s.press(Rune('q'))
	expect(t, s.app.Mode(), ModeNormal)
	expectTrue(t, !s.app.ShouldQuit(), "q in help must not quit")
}

func TestEveryChangeIsSavedAndSurvivesARestart(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune(' '), Rune('a'))
	s.typeStr("later")
	s.press(enter)
	expect(t, s.saved()[0].Status, domain.InProgress)
	reopened := openApp(s.dir)
	expect(t, len(reopened.State.Items), 4)
	expect(t, reopened.State.Items[3].Title, "later")
}

func TestAPullThatShrinksTheListKeepsTheCursorOnARealRow(t *testing.T) {
	s := newSession(t, seed())
	s.press(Rune('G'))
	must(t, openApp(s.dir).store.Save(seed()[:1]))
	s.app.ApplySyncStatus(syncPulled)
	expect(t, len(s.items()), 1)
	expect(t, s.cursor(), 0)
}

func TestANewTaskIsSelectedWhereverSortingPutsIt(t *testing.T) {
	s := newSession(t, seed())
	s.app.State.SortMode = domain.SortTitle
	s.app.refresh()
	s.press(Rune('a'))
	s.typeStr("AAA first alphabetically")
	s.press(enter)
	expect(t, s.items()[s.cursor()].Title, "AAA first alphabetically")
}

func TestTwoInstancesMergeInsteadOfLosingEdits(t *testing.T) {
	dir := t.TempDir()
	must(t, storage.NewJSONStore(dir).Save(seed()))
	first, second := openApp(dir), openApp(dir)
	first.createTask(TaskDraft{ID: newTask, Title: "from first", Workspace: "Work", Priority: domain.Medium})
	second.createTask(TaskDraft{ID: newTask, Title: "from second", Workspace: "Work", Priority: domain.Medium})
	reloaded, err := storage.NewJSONStore(dir).Load()
	must(t, err)
	var titles []string
	for _, it := range reloaded {
		titles = append(titles, it.Title)
	}
	if !slices.Contains(titles, "from first") || !slices.Contains(titles, "from second") {
		t.Fatalf("an edit was lost: %v", titles)
	}
}
