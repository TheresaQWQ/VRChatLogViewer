use crate::vrc::parser::LogItem;

#[derive(Debug, Clone)]
pub enum Message {
    NewLog(LogItem),
    FilterTextChanged(String),
    FilterLevelChanged(Option<LogLevel>),
    ToggleExpand(usize),
    CopyToClipboard(String),
    NextPage,
    PrevPage,
    OpenSettings,
    CloseSettings,
    SaveSettings,
    SettingsFileCountChanged(String),
    SettingsInitialLogsChanged(String),
    SettingsMaxCachedChanged(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    ALL,
    Info,
    Warning,
    Error,
    Debug,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::ALL => write!(f, "All"),
            LogLevel::Info => write!(f, "Info"),
            LogLevel::Warning => write!(f, "Warning"),
            LogLevel::Error => write!(f, "Error"),
            LogLevel::Debug => write!(f, "Debug"),
        }
    }
}

impl LogLevel {
    pub fn to_string(&self) -> String {
        match self {
            LogLevel::ALL => "All".to_string(),
            LogLevel::Info => "Info".to_string(),
            LogLevel::Warning => "Warning".to_string(),
            LogLevel::Error => "Error".to_string(),
            LogLevel::Debug => "Debug".to_string(),
        }
    }

    pub fn all_levels() -> &'static [LogLevel] {
        &[
            LogLevel::ALL,
            LogLevel::Info,
            LogLevel::Warning,
            LogLevel::Error,
            LogLevel::Debug,
        ]
    }
}
