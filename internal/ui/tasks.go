package ui

import (
	"slices"

	"github.com/AmaneKai/hopes/internal/domain"
)

func (a *App) saveTask(d TaskDraft) {
	if d.ID == newTask {
		a.createTask(d)
	} else {
		a.updateTask(d)
	}
	a.mode = ModeNormal
}

func (a *App) updateTask(d TaskDraft) {
	it := a.item(d.ID)
	if it == nil {
		return
	}
	moved := it.Workspace != d.Workspace
	it.Title, it.Description = d.Title, d.Description
	it.Priority, it.Workspace = d.Priority, d.Workspace
	if moved {
		a.commit("Moved to " + d.Workspace)
		return
	}
	a.commit("Task updated")
}

func (a *App) createTask(d TaskDraft) {
	it := domain.NewItem(d.Title, d.Workspace)
	it.Description, it.Priority, it.Status = d.Description, d.Priority, d.Status
	a.State.Items = append(a.State.Items, it)
	if d.Workspace != a.State.Workspace {
		a.openWorkspace(d.Workspace)
	}
	a.commit("Task created")
	a.selectItem(len(a.State.Items) - 1)
}

func (a *App) deleteTask(idx int) {
	it := a.item(idx)
	if it == nil {
		return
	}
	deleted := *it
	a.State.Items = append(a.State.Items[:idx], a.State.Items[idx+1:]...)
	a.State.LastDeleted = &domain.Deleted{Index: idx, Item: deleted}
	a.commit("Deleted \"" + domain.Snippet(deleted.Title, 28) + "\" (press u to undo)")
}

func (a *App) undoDelete() {
	last := a.State.LastDeleted
	if last == nil {
		a.State.SetStatus("Nothing to undo")
		return
	}
	a.State.LastDeleted = nil
	at := min(last.Index, len(a.State.Items))
	if last.Item.Workspace != a.State.Workspace {
		a.openWorkspace(last.Item.Workspace)
	}
	items := append(a.State.Items[:at:at], last.Item)
	a.State.Items = append(items, a.State.Items[at:]...)
	a.commit("Restored \"" + domain.Snippet(last.Item.Title, 28) + "\"")
	a.selectItem(at)
}

func (a *App) toggleStatus(idx int) {
	a.changeStatus(idx, domain.Status.Next, "Status changed to ")
}

func (a *App) shiftStatus(idx int, forward bool) {
	a.changeStatus(idx, pick(forward, domain.Status.Next, domain.Status.Prev), "Shifted task to ")
}

func (a *App) changeStatus(idx int, step func(domain.Status) domain.Status, prefix string) {
	if it := a.item(idx); it != nil {
		it.Status = step(it.Status)
		a.commit(prefix + it.Status.Label())
	}
}

func (a *App) setPriority(idx int, p domain.Priority) {
	if it := a.item(idx); it != nil {
		it.Priority = p
		a.commit("Priority set to " + p.Label())
	}
}

func (a *App) cyclePriority(idx int, forward bool) {
	it := a.item(idx)
	if it == nil {
		return
	}
	if forward {
		a.setPriority(idx, it.Priority.Next())
		return
	}
	a.setPriority(idx, it.Priority.Prev())
}

func (a *App) moveTask(idx int, up bool) {
	if a.State.SortMode != domain.SortNatural {
		a.State.SetStatus("Reordering needs Natural sort")
		return
	}
	visible := a.State.FilteredIndices()
	pos := slices.Index(visible, idx)
	other := pos + 1
	if up {
		other = pos - 1
	}
	if pos < 0 || other < 0 || other >= len(visible) {
		return
	}
	target := visible[other]
	items := a.State.Items
	items[idx], items[target] = items[target], items[idx]
	direction := pick(up, "up", "down")
	a.commit("Moved task " + direction)
	a.selectItem(target)
}
