package ui

import (
	"fmt"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
	"github.com/AmaneKai/hopes/internal/storage"
	"github.com/AmaneKai/hopes/internal/viewstate"
)

type Mode int

const (
	ModeNormal Mode = iota
	ModeCreating
	ModeEditing
	ModeFinding
	ModeCommand
	ModeHelp
)

type App struct {
	State   *domain.AppState
	mode    Mode
	quit    bool
	store   storage.Storage
	sync    *gitsync.Engine
	loadErr error

	table   *tableView
	board   *boardView
	nav     *sidebar
	editor  *editor
	help    *helpPopup
	cmdline *cmdline
}

func New(store storage.Storage, engine *gitsync.Engine) *App {
	items, loadErr := store.Load()
	syncStatus := gitsync.Status{Kind: gitsync.Idle}
	if !engine.Enabled {
		syncStatus.Kind = gitsync.Disabled
	}
	a := &App{
		State:   domain.NewAppState(items, syncStatus),
		store:   store,
		sync:    engine,
		loadErr: loadErr,
		table:   newTableView(len(items) > 0),
		board:   &boardView{},
		nav:     &sidebar{},
		editor:  newEditor(),
		help:    &helpPopup{},
		cmdline: newCmdline(),
	}
	if loadErr != nil {
		a.State.SetStatus(fmt.Sprintf("Could not load tasks (%v) — saving is disabled", loadErr))
	}
	return a
}

func (a *App) RestoreView(v viewstate.ViewState) { v.Apply(a.State) }

func (a *App) SaveViewState() { viewstate.Capture(a.State).Save() }

func (a *App) ShutdownSync() { a.sync.Shutdown() }

func (a *App) Mode() Mode       { return a.mode }
func (a *App) ShouldQuit() bool { return a.quit }

func (a *App) view() taskView {
	if a.State.ViewMode == domain.Kanban {
		return a.board
	}
	return a.table
}

func (a *App) SelectedIndex() (int, bool) { return a.view().selectedIndex(a.State) }

func (a *App) PendingChord() string { return a.view().pendingLabel() }

func (a *App) HandleKey(k Key) {
	switch a.mode {
	case ModeNormal:
		a.normalKey(k)
	case ModeCreating, ModeEditing:
		a.editorKey(k)
	case ModeFinding, ModeCommand:
		a.cmdline.handleKey(k, a)
	case ModeHelp:
		a.help.handleKey(k, a)
	}
}

func (a *App) editorKey(k Key) {
	switch a.editor.handleKey(k, a.State) {
	case formSave:
		a.saveTask(a.editor.draft(a.State))
	case formCancel:
		a.closeModal()
	}
}

func (a *App) Paste(text string) {
	switch a.mode {
	case ModeCreating, ModeEditing:
		a.editor.paste(text)
	case ModeFinding, ModeCommand:
		a.cmdline.paste(text)
	}
}

func (a *App) normalKey(k Key) {
	if !k.IsAny('g', 'd') {
		a.table.clearPending()
		a.board.clearPending()
	}
	if a.frameKey(k) {
		return
	}
	if a.State.Focus == domain.FocusNav && a.nav.handleKey(k, a) {
		return
	}
	if !a.view().handleKey(k, a) {
		a.globalKey(k)
	}
}

func (a *App) frameKey(k Key) bool {
	switch {
	case k.Is('q') || k.IsCtrl('c'):
		a.quit = true
	case k.Code == KeyTab:
		a.cycleWorkspace(true)
	case k.Code == KeyBackTab:
		a.cycleWorkspace(false)
	case k.IsCtrl('w'):
		a.State.Focus = a.State.Focus.Other()
	default:
		return false
	}
	return true
}

var globalKeys = map[rune]func(*App){
	'v': (*App).toggleViewMode,
	's': (*App).cycleSort,
	'h': (*App).focusNav,
	'l': (*App).focusMain,
	'u': (*App).undoDelete,
	'/': (*App).openFind,
	':': (*App).openCommand,
	'?': (*App).openHelp,
	'S': (*App).triggerManualSync,
}

var globalSpecialKeys = map[KeyCode]func(*App){
	KeyLeft:  (*App).focusNav,
	KeyRight: (*App).focusMain,
	KeyEsc:   (*App).clearFilter,
	KeyF5:    (*App).triggerManualSync,
}

func (a *App) globalKey(k Key) {
	if cmd, ok := globalSpecialKeys[k.Code]; ok {
		cmd(a)
		return
	}
	if k.Code != KeyRune || k.Ctrl {
		return
	}
	if cmd, ok := globalKeys[k.Rune]; ok {
		cmd(a)
	}
}
