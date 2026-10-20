use crate::domain::{AppState, NavSection, SortMode, ViewMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// UI choices that survive restarts. Kept out of the data dir so it never gets synced.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ViewState {
    pub view_mode: ViewMode,
    pub sort_mode: SortMode,
    pub nav_section: NavSection,
    pub tag_filter: Option<String>,
}

fn path() -> Option<PathBuf> {
    let mut dir = dirs::state_dir().or_else(dirs::data_local_dir)?;
    dir.push("hopes");
    dir.push("view.json");
    Some(dir)
}

impl ViewState {
    pub fn capture(state: &AppState) -> Self {
        Self {
            view_mode: state.view_mode,
            sort_mode: state.sort_mode,
            nav_section: state.nav_section,
            tag_filter: state.tag_filter.clone(),
        }
    }

    pub fn apply(self, state: &mut AppState) {
        state.view_mode = self.view_mode;
        state.sort_mode = self.sort_mode;
        state.nav_section = self.nav_section;
        state.tag_filter = self.tag_filter.filter(|f| match f.strip_prefix('#') {
            Some(tag) => state.tags().iter().any(|(t, _)| t == tag),
            None => true,
        });
        state.refresh_cache();
    }

    pub fn load() -> Option<Self> {
        let contents = std::fs::read_to_string(path()?).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
