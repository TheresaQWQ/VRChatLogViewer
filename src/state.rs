use crate::config::AppConfig;
use crate::log_entry::LogEntry;
use crate::message::{LogLevel, Message};
use crate::stream::watch_logs_stream;
use crate::view;
use iced::{Element, Subscription, Task};
use regex::RegexBuilder;
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

// ==========================================
// 1. 视图模式枚举
// ==========================================
#[derive(PartialEq)]
pub enum ViewMode {
    Logs,
    Settings,
}

// ==========================================
// 2. 主状态结构体
// ==========================================
pub struct VRCLog {
    // 核心数据
    logs: BTreeSet<LogEntry>,
    filtered_logs: Vec<LogEntry>,
    next_id: usize,

    // 视图与配置
    pub config: AppConfig,
    pub view_mode: ViewMode,

    // 设置页面的临时输入状态
    temp_file_count: String,
    temp_initial_logs: String,
    temp_max_cached: String,

    // 过滤与分页
    filter_text: String,
    filter_level: Option<LogLevel>,
    filter_regex_enabled: bool,
    filter_regex_error: Option<String>,
    current_page: usize,
    items_per_page: usize,
    expanded_rows: HashSet<usize>,
}

impl Default for VRCLog {
    fn default() -> Self {
        let config = AppConfig::load();
        Self {
            logs: BTreeSet::new(),
            filtered_logs: Vec::new(),
            next_id: 0,

            temp_file_count: config.file_count.to_string(),
            temp_initial_logs: config.initial_logs_per_file.to_string(),
            temp_max_cached: config.max_cached_logs.to_string(),

            config,
            view_mode: ViewMode::Logs,

            filter_text: String::new(),
            filter_level: None,
            filter_regex_enabled: false,
            filter_regex_error: None,
            current_page: 0,
            items_per_page: 50,
            expanded_rows: HashSet::new(),
        }
    }
}

impl VRCLog {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewLog(log) => {
                let entry = LogEntry {
                    id: self.next_id,
                    item: Rc::new(log),
                };
                self.next_id += 1;

                self.logs.insert(entry);

                // 使用配置的 max_cached_logs 控制内存占用
                while self.logs.len() > self.config.max_cached_logs {
                    self.logs.pop_last();
                }

                self.apply_filters();
            }
            Message::FilterTextChanged(new_text) => {
                self.filter_text = new_text;
                self.current_page = 0;
                self.apply_filters();
            }
            Message::FilterLevelChanged(new_type) => {
                self.filter_level = new_type;
                self.current_page = 0;
                self.apply_filters();
            }
            Message::FilterRegexToggled(enabled) => {
                self.filter_regex_enabled = enabled;
                self.current_page = 0;
                self.apply_filters();
            }
            Message::NextPage => {
                if self.current_page < self.max_pages() {
                    self.current_page += 1;
                }
            }
            Message::PrevPage => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                }
            }
            Message::ToggleExpand(log_id) => {
                if self.expanded_rows.contains(&log_id) {
                    self.expanded_rows.remove(&log_id);
                } else {
                    self.expanded_rows.insert(log_id);
                }
            }
            Message::CopyToClipboard(text) => {
                return iced::clipboard::write(text);
            }

            // ================== 设置页面事件 ==================
            Message::OpenSettings => {
                self.temp_file_count = self.config.file_count.to_string();
                self.temp_initial_logs = self.config.initial_logs_per_file.to_string();
                self.temp_max_cached = self.config.max_cached_logs.to_string();
                self.view_mode = ViewMode::Settings;
            }
            Message::CloseSettings => {
                self.view_mode = ViewMode::Logs;
            }
            Message::SettingsFileCountChanged(val) => self.temp_file_count = val,
            Message::SettingsInitialLogsChanged(val) => self.temp_initial_logs = val,
            Message::SettingsMaxCachedChanged(val) => self.temp_max_cached = val,
            Message::SaveSettings => {
                // 解析用户输入，如果无效则回退到当前值
                self.config.file_count = self
                    .temp_file_count
                    .parse()
                    .unwrap_or(self.config.file_count);
                self.config.initial_logs_per_file = self
                    .temp_initial_logs
                    .parse()
                    .unwrap_or(self.config.initial_logs_per_file);
                self.config.max_cached_logs = self
                    .temp_max_cached
                    .parse()
                    .unwrap_or(self.config.max_cached_logs);

                self.config.save(); // 持久化到 JSON

                // 应用最新的缓存限制
                while self.logs.len() > self.config.max_cached_logs {
                    self.logs.pop_last();
                }
                self.apply_filters();
                self.view_mode = ViewMode::Logs; // 保存后返回日志视图
            }
        }
        Task::none()
    }

    fn apply_filters(&mut self) {
        self.filtered_logs.clear();
        self.filter_regex_error = None;

        let filter_regex = if self.filter_regex_enabled && !self.filter_text.is_empty() {
            match RegexBuilder::new(&self.filter_text)
                .case_insensitive(true)
                .build()
            {
                Ok(regex) => Some(regex),
                Err(error) => {
                    self.filter_regex_error = Some(format!("Invalid regex: {}", error));
                    return;
                }
            }
        } else {
            None
        };

        let filter_lower = if filter_regex.is_none() {
            self.filter_text.to_lowercase()
        } else {
            String::new()
        };

        for entry in &self.logs {
            let matches_text = if self.filter_text.is_empty() {
                true
            } else if let Some(regex) = &filter_regex {
                regex.is_match(&entry.item.raw)
            } else {
                entry.item.raw.to_lowercase().contains(&filter_lower)
            };

            let matches_level = match self.filter_level {
                None | Some(LogLevel::ALL) => true,
                Some(t) => entry.item.level == t.to_string(),
            };

            if matches_text && matches_level {
                self.filtered_logs.push(entry.clone());
            }
        }
    }

    fn max_pages(&self) -> usize {
        self.filtered_logs.len().saturating_sub(1) / self.items_per_page
    }

    // ==========================================
    // 3. 视图渲染路由
    // ==========================================
    pub fn view(&self) -> Element<'_, Message> {
        match self.view_mode {
            ViewMode::Logs => view::view_logs(
                &self.filtered_logs,
                &self.filter_text,
                &self.filter_level,
                self.filter_regex_enabled,
                self.filter_regex_error.as_deref(),
                self.current_page,
                self.items_per_page,
                &self.expanded_rows,
                self.logs.len(),
            ),
            ViewMode::Settings => view::view_settings(
                &self.temp_file_count,
                &self.temp_initial_logs,
                &self.temp_max_cached,
            ),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(watch_logs_stream)
    }
}
