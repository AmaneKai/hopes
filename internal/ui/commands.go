package ui

import (
	"errors"
	"fmt"
	"os"
	"slices"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
	"github.com/AmaneKai/hopes/internal/storage"
)

func (a *App) focusNav()  { a.State.Focus = domain.FocusNav }
func (a *App) focusMain() { a.State.Focus = domain.FocusMain }

func (a *App) cycleSort() {
	a.State.SortMode = a.State.SortMode.Next()
	a.refresh()
	a.State.SetStatus("Sort: " + a.State.SortMode.Label())
}

func (a *App) refresh() {
	a.State.RefreshCache()
	a.table.clamp(a.State)
}

func (a *App) openCreate(status domain.Status) {
	a.editor.openCreate(a.State.Workspace, status)
	a.mode = ModeCreating
}

func (a *App) openFind() {
	a.cmdline.open(true, a.State.Filter)
	a.mode = ModeFinding
}

func (a *App) openCommand() {
	a.cmdline.open(false, "")
	a.mode = ModeCommand
}

func (a *App) openHelp() {
	a.help.scroll = 0
	a.mode = ModeHelp
}

func (a *App) closeModal() {
	a.help.scroll = 0
	a.mode = ModeNormal
}

func (a *App) setFilter(query string) {
	a.State.Filter, a.State.HasFilter = query, query != ""
	a.filterChanged()
}

func (a *App) jumpTo(index int) {
	ws := a.State.Workspaces()
	if index >= 0 && index < len(ws) && ws[index].Name != a.State.Workspace {
		a.switchWorkspace(ws[index].Name)
	}
}

func (a *App) toggleViewMode() {
	if a.State.ViewMode == domain.Table {
		a.State.ViewMode = domain.Kanban
		a.State.SetStatus("Switched to Kanban Board view")
		return
	}
	a.State.ViewMode = domain.Table
	a.State.SetStatus("Switched to Table Grid view")
}

func (a *App) openEdit(idx int) {
	if it := a.item(idx); it != nil {
		a.editor.openEdit(idx, *it)
		a.mode = ModeEditing
	}
}

func (a *App) item(idx int) *domain.Item {
	if idx < 0 || idx >= len(a.State.Items) {
		return nil
	}
	return &a.State.Items[idx]
}

func (a *App) persist() error {
	if a.loadErr != nil {
		return fmt.Errorf("not saved, tasks failed to load: %v", a.loadErr)
	}
	err := a.store.Save(a.State.Items)
	if errors.Is(err, storage.ErrChangedOnDisk) {
		err = a.mergeWithDisk()
	}
	if err != nil {
		return fmt.Errorf("save failed: %v", err)
	}
	a.saved = domain.CloneItems(a.State.Items)
	a.sync.RequestPush()
	return nil
}

func (a *App) mergeWithDisk() error {
	theirs, err := a.store.Load()
	if err != nil {
		return err
	}
	a.State.Items = domain.Merge(a.saved, a.State.Items, theirs)
	return a.store.Save(a.State.Items)
}

func (a *App) mergeAfterConflict(backup string) {
	mine, err := storage.ReadItemsFile(backup)
	if err != nil {
		a.State.SetStatus("Sync conflict — could not read local backup " + backup)
		return
	}
	base, _ := storage.ReadItemsFile(gitsync.BasePathFor(backup))
	theirs, err := a.store.Load()
	if err != nil {
		a.State.SetStatus("Sync conflict — could not read remote tasks: " + err.Error())
		return
	}
	a.State.Items = domain.Merge(base, mine, theirs)
	a.saved = theirs
	a.commit("Sync conflict merged with remote — backup kept at " + backup)
	_ = os.Remove(gitsync.BasePathFor(backup))
}

func (a *App) commit(status string) {
	err := a.persist()
	a.refresh()
	if err != nil {
		status = "Warning: " + err.Error()
	}
	a.State.SetStatus(status)
}

func (a *App) selectItem(idx int) {
	visible := a.State.FilteredIndices()
	if pos := slices.Index(visible, idx); pos >= 0 {
		a.table.moveTo(pos, len(visible))
	}
}

func (a *App) filterChanged() {
	a.State.RefreshCache()
	a.table.moveTo(0, len(a.State.FilteredIndices()))
}

func (a *App) openWorkspace(name string) {
	a.State.Workspace = name
	a.State.Filter, a.State.HasFilter = "", false
}

func (a *App) switchWorkspace(name string) {
	name = strings.TrimSpace(name)
	if name == "" {
		a.State.SetStatus("Usage: :ws <name>")
		return
	}
	existing, found := a.State.FindWorkspace(name)
	if !found {
		existing = name
		a.State.SetStatus(fmt.Sprintf("New workspace %s — press a to add a task", quote(name)))
	}
	a.openWorkspace(existing)
	a.filterChanged()
}

func (a *App) cycleWorkspace(forward bool) {
	next, ok := a.State.WorkspaceAtOffset(forward)
	if !ok {
		a.State.SetStatus("Only one workspace — :ws <name> makes another")
		return
	}
	a.switchWorkspace(next)
}

func (a *App) renameWorkspace(name string) {
	name = strings.TrimSpace(name)
	old := a.State.Workspace
	if name == "" {
		a.State.SetStatus("Usage: :rename <name>")
		return
	}
	if existing, found := a.State.FindWorkspace(name); found && existing != old {
		a.State.SetStatus(fmt.Sprintf("A workspace named %s already exists", quote(existing)))
		return
	}
	moved := 0
	for i := range a.State.Items {
		if a.State.Items[i].Workspace == old {
			a.State.Items[i].Workspace = name
			moved++
		}
	}
	a.State.Workspace = name
	a.commitIf(moved > 0, fmt.Sprintf("Renamed %s to %s", quote(old), quote(name)))
}

func (a *App) commitIf(changed bool, status string) {
	if changed {
		a.commit(status)
		return
	}
	a.refresh()
	a.State.SetStatus(status)
}

func quote(s string) string { return "“" + s + "”" }

func (a *App) clearFilter() {
	if !a.State.HasFilter {
		return
	}
	a.State.Filter, a.State.HasFilter = "", false
	a.filterChanged()
	a.State.SetStatus("Search cleared")
}

func (a *App) runCommand(line string) {
	name, arg, _ := strings.Cut(strings.TrimSpace(line), " ")
	arg = strings.TrimSpace(arg)
	switch name {
	case "ws", "workspace":
		a.switchWorkspace(arg)
	case "rename":
		a.renameWorkspace(arg)
	case "sync", "s":
		a.triggerManualSync()
	case "q", "quit", "wq":
		a.quit = true
	case "w":
		a.State.SetStatus("Already saved automatically")
	case "help", "h":
		a.openHelp()
		return
	case "":
	default:
		a.State.SetStatus("Unknown command: " + name)
	}
	a.mode = ModeNormal
}

func (a *App) triggerManualSync() {
	if !a.sync.Enabled {
		a.State.SetStatus("Sync is disabled (no Git repo configured)")
		return
	}
	a.sync.ForceSync()
	a.State.SetStatus("Manual sync requested")
}

func (a *App) ApplySyncStatus(status gitsync.Status) {
	switch status.Kind {
	case gitsync.UpdatedRemoteData:
		a.reloadFromDisk()
		a.State.SetStatus("Pulled remote changes")
	case gitsync.Offline:
		a.State.SetStatus("Offline — will sync when connection returns")
	case gitsync.Conflict:
		a.mergeAfterConflict(status.Detail)
	case gitsync.Failed:
		a.State.SetStatus("Sync error: " + status.Detail)
	}
	a.State.SyncStatus = status
}

func (a *App) reloadFromDisk() {
	items, err := a.store.Load()
	if err != nil {
		return
	}
	a.loadErr = nil
	a.State.Items = items
	a.saved = domain.CloneItems(items)
	a.refresh()
}
