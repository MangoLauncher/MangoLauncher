use std::collections::VecDeque;
use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub struct MinecraftVersion {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
}

pub struct App {
    pub should_quit: bool,
    pub versions: VecDeque<MinecraftVersion>,
    pub state: ListState,
}

impl App {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            should_quit: false,
            versions: VecDeque::new(),
            state,
        }
    }

    pub fn next(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected < self.versions.len().saturating_sub(1) {
                self.state.select(Some(selected + 1));
            }
        }
    }

    pub fn previous(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
            }
        }
    }
} 