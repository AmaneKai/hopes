package ui

import (
	"fmt"
	"slices"
	"strings"

	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

func (a *App) apply(act Action) {
	if a.applyView(act) || a.applyWorkspace(act) || a.applyTask(act) {
		return
	}
	a.applySync(act)
}

func (a *App) applyView(act Action) bool {
	switch act := act.(type) {
	case Quit:
		a.quit = true
	case ToggleFocus:
		a.State.Focus = a.State.Focus.Other()
	case SetFocus:
		a.State.Focus = act.Pane
	case ToggleViewMode:
		a.toggleViewMode()
	case CycleSort:
		a.State.SortMode = a.State.SortMode.Next()
		a.State.RefreshCache()
		a.State.SetStatus("Sort: " + a.State.SortMode.Label())
	case OpenCreateModal:
		a.mode = ModeCreating
	case OpenEditModal:
		a.openEdit(act.Index)
	case OpenFind:
		a.mode = ModeFinding
	case OpenCommand:
		a.mode = ModeCommand
	case OpenHelp:
		a.mode = ModeHelp
	case CloseModal:
		a.mode = ModeNormal
	case RunCommand:
		a.runCommand(act.Line)
	default:
		return false
	}
	return true
}

func (a *App) applyWorkspace(act Action) bool {
	switch act := act.(type) {
	case SwitchWorkspace:
		a.switchWorkspace(act.Name)
	case CycleWorkspace:
		a.cycleWorkspace(act.Forward)
	case RenameWorkspace:
		a.renameWorkspace(act.Name)
	case SetFilter:
		a.State.Filter, a.State.HasFilter = act.Query, act.Query != ""
		a.filterChanged()
	case ClearFilter:
		a.clearFilter()
	default:
		return false
	}
	return true
}

func (a *App) applyTask(act Action) bool {
	switch act := act.(type) {
	case SaveTask:
		a.saveTask(act.Draft)
	case DeleteTask:
		a.deleteTask(act.Index)
	case UndoDelete:
		a.undoDelete()
	case ToggleTaskStatus:
		a.changeStatus(act.Index, domain.Status.Next, "Status changed to ")
	case ShiftTaskStatus:
		a.changeStatus(act.Index, statusStep(act.Forward), "Shifted task to ")
	case SetPriority:
		a.setPriority(act.Index, act.Priority)
	case CyclePriority:
		a.cyclePriority(act.Index, act.Forward)
	case MoveTask:
		a.moveTask(act.Index, act.Up)
	default:
		return false
	}
	return true
}

func (a *App) applySync(act Action) {
	switch act := act.(type) {
	case SyncRequested:
		a.sync.RequestPush()
	case ForceSync:
		a.triggerManualSync()
	case SyncStatusChanged:
		a.applySyncStatus(act.Status)
	}
}

func statusStep(forward bool) func(domain.Status) domain.Status {
	if forward {
		return domain.Status.Next
	}
	return domain.Status.Prev
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
	if idx >= 0 && idx < len(a.State.Items) {
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
	if err := a.store.Save(a.State.Items); err != nil {
		return fmt.Errorf("save failed: %v", err)
	}
	a.Send(SyncRequested{})
	return nil
}

func (a *App) commit(status string) {
	err := a.persist()
	a.State.RefreshCache()
	if err != nil {
		status = "Warning: " + err.Error()
	}
	a.State.SetStatus(status)
}

func (a *App) selectItem(idx int) {
	if pos := slices.Index(a.State.FilteredIndices(), idx); pos >= 0 {
		a.Send(SelectIndex{pos})
	}
}

func (a *App) filterChanged() {
	a.State.RefreshCache()
	a.Send(SelectIndex{0})
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
	a.State.RefreshCache()
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
		a.Send(OpenHelp{})
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

func (a *App) applySyncStatus(status gitsync.Status) {
	switch status.Kind {
	case gitsync.UpdatedRemoteData:
		a.reloadFromDisk()
		a.State.SetStatus("Pulled remote changes")
	case gitsync.Offline:
		a.State.SetStatus("Offline — will sync when connection returns")
	case gitsync.Conflict:
		a.reloadFromDisk()
		a.State.SetStatus("Sync conflict — local backup saved to " + status.Detail)
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
	a.State.RefreshCache()
	a.Send(DataReloaded{})
}
