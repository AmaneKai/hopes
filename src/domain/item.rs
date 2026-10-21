use super::{priority::Priority, status::Status};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub priority: Priority,
    pub status: Status,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<NaiveDate>,
}

impl Item {
    pub fn new(title: String, tags: Vec<String>) -> Self {
        Self {
            title,
            description: String::new(),
            priority: Priority::Medium,
            status: Status::Todo,
            tags,
            due: None,
        }
    }

    /// Open tasks past their due date.
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        self.status != Status::Complete && self.due.is_some_and(|d| d < today)
    }

    #[allow(dead_code)]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}
