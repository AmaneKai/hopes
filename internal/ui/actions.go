package ui

import (
	"github.com/AmaneKai/hopes/internal/domain"
	"github.com/AmaneKai/hopes/internal/gitsync"
)

type Action interface{}

type TaskDraft struct {
	ID          int
	Title       string
	Description string
	Priority    domain.Priority
	Workspace   string
	Status      domain.Status
}

type (
	Quit             struct{}
	ToggleFocus      struct{}
	SetFocus         struct{ Pane domain.FocusPane }
	ToggleViewMode   struct{}
	CycleSort        struct{}
	OpenCreateModal  struct{ Status domain.Status }
	OpenEditModal    struct{ Index int }
	OpenFind         struct{}
	OpenCommand      struct{}
	OpenHelp         struct{}
	CloseModal       struct{}
	RunCommand       struct{ Line string }
	SwitchWorkspace  struct{ Name string }
	CycleWorkspace   struct{ Forward bool }
	RenameWorkspace  struct{ Name string }
	SetFilter        struct{ Query string }
	ClearFilter      struct{}
	SelectNext       struct{}
	SelectPrev       struct{}
	SelectIndex      struct{ Position int }
	SaveTask         struct{ Draft TaskDraft }
	DeleteTask       struct{ Index int }
	UndoDelete       struct{}
	ToggleTaskStatus struct{ Index int }
	ShiftTaskStatus  struct {
		Index   int
		Forward bool
	}
	SetPriority struct {
		Index    int
		Priority domain.Priority
	}
	CyclePriority struct {
		Index   int
		Forward bool
	}
	MoveTask struct {
		Index int
		Up    bool
	}
	SyncRequested     struct{}
	ForceSync         struct{}
	SyncStatusChanged struct{ Status gitsync.Status }
	DataReloaded      struct{}
	Batch             []Action
)

const newTask = -1
