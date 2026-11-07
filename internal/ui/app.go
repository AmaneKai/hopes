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
	queue   []Action
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
func (a *App) Send(act Action)  { a.queue = append(a.queue, act) }

func (a *App) ProcessActions() {
	for len(a.queue) > 0 {
		act := a.queue[0]
		a.queue = a.queue[1:]
		a.process(act)
	}
}

func (a *App) components() []component {
	return []component{a.nav, a.table, a.board, a.editor, a.help, a.cmdline}
}

func (a *App) process(act Action) {
	if batch, ok := act.(Batch); ok {
		for _, inner := range batch {
			a.process(inner)
		}
		return
	}
	a.apply(act)
	for _, c := range a.components() {
		c.update(act, a.State)
	}
}

func (a *App) view() taskView {
	if a.State.ViewMode == domain.Kanban {
		return a.board
	}
	return a.table
}

func (a *App) SelectedIndex() (int, bool) { return a.view().selectedIndex(a.State) }

func (a *App) PendingChord() string { return a.view().pendingLabel() }

func (a *App) HandleKey(k Key) {
	var act Action
	switch a.mode {
	case ModeNormal:
		act, _ = a.normalKey(k)
	case ModeCreating, ModeEditing:
		act, _ = a.editor.handleKey(k, a.State)
	case ModeFinding, ModeCommand:
		act, _ = a.cmdline.handleKey(k, a.State)
	case ModeHelp:
		act, _ = a.help.handleKey(k, a.State)
	}
	if act != nil {
		a.Send(act)
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

func (a *App) normalKey(k Key) (Action, bool) {
	if !k.IsAny('g', 'd') {
		a.table.clearPending()
		a.board.clearPending()
	}
	if act, ok := frameKey(k); ok {
		return act, true
	}
	if a.State.Focus == domain.FocusNav {
		if act, ok := a.nav.handleKey(k, a.State); ok {
			return act, true
		}
	}
	if act, ok := a.view().handleKey(k, a.State); ok {
		return act, true
	}
	return globalKey(k)
}

func frameKey(k Key) (Action, bool) {
	switch {
	case k.Is('q') || k.IsCtrl('c'):
		return Quit{}, true
	case k.Code == KeyTab:
		return CycleWorkspace{Forward: true}, true
	case k.Code == KeyBackTab:
		return CycleWorkspace{Forward: false}, true
	case k.IsCtrl('w'):
		return ToggleFocus{}, true
	}
	return nil, false
}

var globalKeys = map[rune]Action{
	'v': ToggleViewMode{},
	's': CycleSort{},
	'h': SetFocus{domain.FocusNav},
	'l': SetFocus{domain.FocusMain},
	'u': UndoDelete{},
	'/': OpenFind{},
	':': OpenCommand{},
	'?': OpenHelp{},
	'S': ForceSync{},
}

var globalSpecialKeys = map[KeyCode]Action{
	KeyLeft:  SetFocus{domain.FocusNav},
	KeyRight: SetFocus{domain.FocusMain},
	KeyEsc:   ClearFilter{},
	KeyF5:    ForceSync{},
}

func globalKey(k Key) (Action, bool) {
	if act, ok := globalSpecialKeys[k.Code]; ok {
		return act, true
	}
	if k.Code != KeyRune || k.Ctrl {
		return nil, false
	}
	act, ok := globalKeys[k.Rune]
	return act, ok
}
