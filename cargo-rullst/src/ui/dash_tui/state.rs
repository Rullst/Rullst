use std::collections::VecDeque;

const LOG_CAPACITY: usize = 1_000;
const SEARCH_CAPACITY: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogEntry {
    pub level: LogLevel,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LogFilter {
    All,
    WarningsAndErrors,
    Errors,
}

impl LogFilter {
    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::WarningsAndErrors,
            Self::WarningsAndErrors => Self::Errors,
            Self::Errors => Self::All,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::WarningsAndErrors => "warnings + errors",
            Self::Errors => "errors",
        }
    }

    const fn accepts(self, level: LogLevel) -> bool {
        match self {
            Self::All => true,
            Self::WarningsAndErrors => matches!(level, LogLevel::Warning | LogLevel::Error),
            Self::Errors => matches!(level, LogLevel::Error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FocusPane {
    Application,
    System,
}

impl FocusPane {
    pub const fn next(self) -> Self {
        match self {
            Self::Application => Self::System,
            Self::System => Self::Application,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ServerStatus {
    Starting,
    Ready,
    Unverified,
    Exited { success: bool, code: Option<i32> },
}

impl ServerStatus {
    pub fn label(self) -> String {
        match self {
            Self::Starting => "STARTING".to_string(),
            Self::Ready => "READY".to_string(),
            Self::Unverified => "UNVERIFIED".to_string(),
            Self::Exited {
                success: true,
                code,
            } => format!("EXITED ({})", display_exit_code(code)),
            Self::Exited {
                success: false,
                code,
            } => format!("FAILED ({})", display_exit_code(code)),
        }
    }
}

fn display_exit_code(code: Option<i32>) -> String {
    code.map_or_else(|| "signal".to_string(), |value| value.to_string())
}

#[derive(Debug)]
pub(super) struct App {
    app_logs: VecDeque<LogEntry>,
    system_logs: VecDeque<String>,
    pub app_scroll_from_bottom: usize,
    pub system_scroll_from_bottom: usize,
    pub focus: FocusPane,
    pub filter: LogFilter,
    pub search_query: String,
    pub search_editing: bool,
    pub action_notice: Option<String>,
    pub server_status: ServerStatus,
    pub migration_running: bool,
    pub studio_probe_running: bool,
    pub tick_count: usize,
    pub port: u16,
    pub hmr_enabled: bool,
    pub database_profile: String,
    pub start_time: std::time::Instant,
    pub colors_enabled: bool,
    pub animations_enabled: bool,
}

impl App {
    pub fn new(
        port: u16,
        hmr_enabled: bool,
        database_profile: String,
        colors_enabled: bool,
        animations_enabled: bool,
    ) -> Self {
        Self {
            app_logs: VecDeque::new(),
            system_logs: VecDeque::new(),
            app_scroll_from_bottom: 0,
            system_scroll_from_bottom: 0,
            focus: FocusPane::Application,
            filter: LogFilter::All,
            search_query: String::new(),
            search_editing: false,
            action_notice: None,
            server_status: ServerStatus::Starting,
            migration_running: false,
            studio_probe_running: false,
            tick_count: 0,
            port,
            hmr_enabled,
            database_profile,
            start_time: std::time::Instant::now(),
            colors_enabled,
            animations_enabled,
        }
    }

    pub fn push_app(&mut self, level: LogLevel, text: String) {
        push_bounded(&mut self.app_logs, LogEntry { level, text });
    }

    pub fn push_system(&mut self, text: String) {
        push_bounded(&mut self.system_logs, text);
    }

    pub fn app_logs(&self) -> Vec<&LogEntry> {
        let query = self.search_query.to_ascii_lowercase();
        self.app_logs
            .iter()
            .filter(|entry| {
                self.filter.accepts(entry.level)
                    && (query.is_empty() || entry.text.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn system_logs(&self) -> Vec<&str> {
        let query = self.search_query.to_ascii_lowercase();
        self.system_logs
            .iter()
            .filter(|entry| query.is_empty() || entry.to_ascii_lowercase().contains(&query))
            .map(String::as_str)
            .collect()
    }

    pub fn scroll_older(&mut self, amount: usize) {
        let scroll = match self.focus {
            FocusPane::Application => &mut self.app_scroll_from_bottom,
            FocusPane::System => &mut self.system_scroll_from_bottom,
        };
        *scroll = scroll.saturating_add(amount).min(LOG_CAPACITY);
    }

    pub fn scroll_newer(&mut self, amount: usize) {
        let scroll = match self.focus {
            FocusPane::Application => &mut self.app_scroll_from_bottom,
            FocusPane::System => &mut self.system_scroll_from_bottom,
        };
        *scroll = scroll.saturating_sub(amount);
    }

    pub fn scroll_to_latest(&mut self) {
        match self.focus {
            FocusPane::Application => self.app_scroll_from_bottom = 0,
            FocusPane::System => self.system_scroll_from_bottom = 0,
        }
    }

    pub fn clear_logs(&mut self) {
        self.app_logs.clear();
        self.system_logs.clear();
        self.app_scroll_from_bottom = 0;
        self.system_scroll_from_bottom = 0;
    }

    pub fn append_search(&mut self, character: char) {
        if !character.is_control() && self.search_query.chars().count() < SEARCH_CAPACITY {
            self.search_query.push(character);
        }
    }

    pub fn backspace_search(&mut self) {
        self.search_query.pop();
    }
}

pub(super) fn scroll_position(total_lines: usize, visible_lines: usize, from_bottom: usize) -> u16 {
    let maximum = total_lines.saturating_sub(visible_lines);
    maximum
        .saturating_sub(from_bottom.min(maximum))
        .min(u16::MAX as usize) as u16
}

fn push_bounded<T>(entries: &mut VecDeque<T>, entry: T) {
    if entries.len() == LOG_CAPACITY {
        entries.pop_front();
    }
    entries.push_back(entry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_remain_bounded_and_keep_the_newest_entries() {
        let mut app = App::new(3_000, true, "not configured".to_string(), true, true);
        for index in 0..(LOG_CAPACITY + 5) {
            app.push_app(LogLevel::Info, format!("entry-{index}"));
        }

        assert_eq!(app.app_logs.len(), LOG_CAPACITY);
        assert_eq!(
            app.app_logs.front().map(|entry| entry.text.as_str()),
            Some("entry-5")
        );
        assert_eq!(
            app.app_logs.back().map(|entry| entry.text.as_str()),
            Some("entry-1004")
        );
    }

    #[test]
    fn scrolling_is_saturating_for_empty_and_short_logs() {
        assert_eq!(scroll_position(0, 20, usize::MAX), 0);
        assert_eq!(scroll_position(5, 20, usize::MAX), 0);
        assert_eq!(scroll_position(100, 20, 0), 80);
        assert_eq!(scroll_position(100, 20, 10), 70);
        assert_eq!(scroll_position(100, 20, usize::MAX), 0);
    }

    #[test]
    fn filtering_and_search_are_composable() {
        let mut app = App::new(3_000, true, "SQLite configured".to_string(), true, true);
        app.push_app(LogLevel::Info, "server ready".to_string());
        app.push_app(LogLevel::Warning, "slow query".to_string());
        app.push_app(LogLevel::Error, "query failed".to_string());

        app.filter = LogFilter::WarningsAndErrors;
        app.search_query = "query".to_string();
        let visible = app.app_logs();
        assert_eq!(visible.len(), 2);

        app.filter = LogFilter::Errors;
        let visible = app.app_logs();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].text, "query failed");
    }

    #[test]
    fn search_input_has_a_hard_character_limit() {
        let mut app = App::new(3_000, true, "not configured".to_string(), true, true);
        for _ in 0..(SEARCH_CAPACITY + 10) {
            app.append_search('a');
        }
        assert_eq!(app.search_query.chars().count(), SEARCH_CAPACITY);
        app.backspace_search();
        assert_eq!(app.search_query.chars().count(), SEARCH_CAPACITY - 1);
    }
}
