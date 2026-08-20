use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    Urgent,
    High,
    Medium,
    Low,
}

#[allow(dead_code)]
impl Priority {
    pub const ALL: [Priority; 4] = [
        Priority::Urgent,
        Priority::High,
        Priority::Medium,
        Priority::Low,
    ];

    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Urgent => Self::High,
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Urgent,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Urgent => Self::Low,
            Self::High => Self::Urgent,
            Self::Medium => Self::High,
            Self::Low => Self::Medium,
        }
    }

    #[inline(always)]
    pub fn color(&self) -> Color {
        match self {
            Self::Urgent => Color::LightRed,
            Self::High => Color::Red,
            Self::Medium => Color::Yellow,
            Self::Low => Color::DarkGray,
        }
    }

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Urgent => "URGENT",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    #[inline(always)]
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Urgent => "URG",
            Self::High => "HIGH",
            Self::Medium => "MED",
            Self::Low => "LOW",
        }
    }

    #[inline(always)]
    pub fn from_digit(d: char) -> Option<Self> {
        match d {
            '1' => Some(Self::Urgent),
            '2' => Some(Self::High),
            '3' => Some(Self::Medium),
            '4' => Some(Self::Low),
            _ => None,
        }
    }
}
