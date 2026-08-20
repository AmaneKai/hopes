use super::{priority::Priority, status::Status};
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
}

impl Item {
    pub fn new(title: String, tags: Vec<String>) -> Self {
        Self {
            title,
            description: String::new(),
            priority: Priority::Medium,
            status: Status::Todo,
            tags,
        }
    }

    #[allow(dead_code)]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}
