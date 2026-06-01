//! TUI state machine.
//!
//! Pure logic for scrolling, filtering, and URL selection — no rendering or
//! terminal I/O. The binary's `tui` module handles drawing and events.

use crate::model::{Match, Verdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
}

pub struct App<'a> {
    idea: &'a str,
    verdict: &'a Verdict,
    matches: &'a [Match],
    cursor: usize,
    filter: String,
    visible: Vec<usize>,
    mode: Mode,
    quit: bool,
}

impl<'a> App<'a> {
    pub fn new(idea: &'a str, verdict: &'a Verdict, matches: &'a [Match]) -> Self {
        let visible = (0..matches.len()).collect();
        Self {
            idea,
            verdict,
            matches,
            cursor: 0,
            filter: String::new(),
            visible,
            mode: Mode::Normal,
            quit: false,
        }
    }

    pub fn idea(&self) -> &str {
        self.idea
    }

    pub fn verdict(&self) -> &Verdict {
        self.verdict
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn filter_text(&self) -> &str {
        &self.filter
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn visible_matches(&self) -> Vec<&Match> {
        self.visible.iter().map(|&i| &self.matches[i]).collect()
    }

    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }

    pub fn scroll_down(&mut self) {
        let max = self.visible.len().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn enter_filter(&mut self) {
        self.mode = Mode::Filter;
        self.cursor = 0;
    }

    pub fn confirm_filter(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn exit_filter(&mut self) {
        self.mode = Mode::Normal;
        self.filter.clear();
        self.recompute_visible();
        self.cursor = 0;
    }

    pub fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.recompute_visible();
        self.clamp_cursor();
    }

    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.recompute_visible();
        self.clamp_cursor();
    }

    pub fn selected_url(&self) -> Option<&str> {
        self.visible
            .get(self.cursor)
            .map(|&i| self.matches[i].url.as_str())
    }

    fn recompute_visible(&mut self) {
        if self.filter.is_empty() {
            self.visible = (0..self.matches.len()).collect();
        } else {
            let lower = self.filter.to_lowercase();
            self.visible = self
                .matches
                .iter()
                .enumerate()
                .filter(|(_, m)| {
                    m.name.to_lowercase().contains(&lower)
                        || m.description.to_lowercase().contains(&lower)
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    fn clamp_cursor(&mut self) {
        if self.visible.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.visible.len() {
            self.cursor = self.visible.len() - 1;
        }
    }
}
