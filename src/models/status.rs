use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Todo,
    InProgress,
    Complete,
}

#[allow(dead_code)]
impl Status {
    pub const ALL: [Status; 3] = [Status::Todo, Status::InProgress, Status::Complete];

    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Todo => Self::InProgress,
            Self::InProgress => Self::Complete,
            Self::Complete => Self::Todo,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Todo => Self::Complete,
            Self::InProgress => Self::Todo,
            Self::Complete => Self::InProgress,
        }
    }

    #[inline(always)]
    pub fn badge(&self) -> (&'static str, Color) {
        match self {
            Self::Todo => ("\u{25cb}", Color::DarkGray),
            Self::InProgress => ("\u{25d0}", Color::Cyan),
            Self::Complete => ("\u{2713}", Color::Green),
        }
    }

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Todo => "To Do",
            Self::InProgress => "In Progress",
            Self::Complete => "Done",
        }
    }

    #[inline(always)]
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::InProgress => "WIP",
            Self::Complete => "DONE",
        }
    }
}
