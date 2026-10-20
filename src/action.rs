use crate::{
    domain::{FocusPane, NavSection, Priority, Status},
    systems::sync::SyncStatus,
};

#[derive(Clone, Debug)]
pub enum NavTarget {
    Section(NavSection),
    Tag(String),
}

#[derive(Clone, Debug)]
pub struct TaskDraft {
    pub id: Option<usize>,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub status: Status,
}

#[derive(Clone, Debug)]
pub enum Action {
    Quit,

    NextPane,
    PrevPane,
    SetFocus(FocusPane),
    ToggleViewMode,
    CycleSort,

    OpenCreateModal { status: Status },
    OpenEditModal(usize),
    OpenFind,
    OpenCommand,
    OpenHelp,
    CloseModal,
    RunCommand(String),

    ApplyNav(NavTarget),
    SetFilter(Option<String>),
    CycleTagFilter,
    ClearFilter,
    ResetNavCursor,

    SelectNext,
    SelectPrev,
    SelectIndex(usize),

    SaveTask(TaskDraft),
    DeleteTask(usize),
    UndoDelete,
    ToggleTaskStatus(usize),
    ShiftTaskStatus { idx: usize, forward: bool },
    SetPriority { idx: usize, priority: Priority },
    CyclePriority { idx: usize, forward: bool },
    MoveTaskUp(usize),
    MoveTaskDown(usize),

    SyncRequested,
    ForceSync,
    SyncStatus(SyncStatus),
    DataReloaded,

    Batch(Vec<Action>),
}
