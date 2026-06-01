//! TUI state machine.
//!
//! Pure logic for scrolling, filtering, and URL selection — no rendering or
//! terminal I/O. The binary's `tui` module handles drawing and events.

use crate::model::{Match, Verdict};

/// How many matches to show before the user presses "show more".
pub const DEFAULT_PAGE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
    Help,
}

pub struct App<'a> {
    idea: &'a str,
    verdict: &'a Verdict,
    matches: &'a [Match],
    cursor: usize,
    filter: String,
    visible: Vec<usize>,
    mode: Mode,
    expanded: bool,
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
            expanded: false,
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

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn visible_matches(&self) -> Vec<&Match> {
        self.visible.iter().map(|&i| &self.matches[i]).collect()
    }

    /// The matches to render — respects page size unless expanded.
    pub fn displayed_matches(&self) -> Vec<&Match> {
        let limit = self.display_limit();
        self.visible
            .iter()
            .take(limit)
            .map(|&i| &self.matches[i])
            .collect()
    }

    /// True when there are more matches beyond the current page.
    pub fn has_more(&self) -> bool {
        !self.expanded && self.visible.len() > DEFAULT_PAGE
    }

    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
        self.clamp_cursor();
    }

    pub fn scroll_down(&mut self) {
        let max = self.display_limit().saturating_sub(1);
        if self.cursor < max {
            self.cursor += 1;
        } else {
            self.cursor = 0;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.cursor == 0 {
            self.cursor = self.display_limit().saturating_sub(1);
        } else {
            self.cursor -= 1;
        }
    }

    pub fn total_matches(&self) -> usize {
        self.matches.len()
    }

    pub fn scroll_to_top(&mut self) {
        self.cursor = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.cursor = self.display_limit().saturating_sub(1);
    }

    pub fn toggle_help(&mut self) {
        self.mode = if self.mode == Mode::Help {
            Mode::Normal
        } else {
            Mode::Help
        };
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
        let limit = self.display_limit();
        self.visible
            .iter()
            .take(limit)
            .nth(self.cursor)
            .map(|&i| self.matches[i].url.as_str())
    }

    fn display_limit(&self) -> usize {
        if self.expanded {
            self.visible.len()
        } else {
            self.visible.len().min(DEFAULT_PAGE)
        }
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
        let limit = self.display_limit();
        if limit == 0 {
            self.cursor = 0;
        } else if self.cursor >= limit {
            self.cursor = limit - 1;
        }
    }
}
