use crate::message::{LogLevel, Message};
use crate::stream::watch_logs_stream;
use crate::vrc;
use iced::widget::{
    button, container, pick_list, scrollable, text_input, Column, Container, Row, Text,
};
use iced::{alignment, Element, Length, Padding, Subscription, Task};
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

// 辅助函数：安全地截断过长的字符串 (按字符而不是字节截断，防止中文字符崩溃)
fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

// 包装器：为日志分配唯一 ID，并配置专供 BTreeSet 使用的排序规则
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub id: usize,
    pub item: Rc<vrc::parser::LogItem>,
}

impl PartialEq for LogEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for LogEntry {}

impl PartialOrd for LogEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 首先按时间戳倒序排列 (最新的日志在最前面)
        let cmp = other.item.timestamp.cmp(&self.item.timestamp);
        if cmp == std::cmp::Ordering::Equal {
            // 如果时间戳相同(同一秒发生的日志)，按 ID 倒序排列确保不会被 BTreeSet 覆盖，且新日志靠前
            other.id.cmp(&self.id)
        } else {
            cmp
        }
    }
}

pub struct VRCLog {
    logs: BTreeSet<LogEntry>,       // 使用平衡二叉树存储，实现自动排序
    filtered_logs: Vec<LogEntry>,   // 存储过滤后的日志引用 (通过 Rc 克隆，开销极低)
    next_id: usize,                 // 唯一递增标识符
    filter_text: String,
    filter_level: Option<LogLevel>,
    current_page: usize,
    items_per_page: usize,
    expanded_rows: HashSet<usize>,  // 现在直接存储日志的唯一 id，不再需要处理索引偏移
}

impl Default for VRCLog {
    fn default() -> Self {
        Self {
            logs: BTreeSet::new(),
            filtered_logs: Vec::new(),
            next_id: 0,
            filter_text: String::new(),
            filter_level: None,
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
                
                // BTreeSet 插入时 $O(\log N)$ 自动排序
                self.logs.insert(entry);

                if self.logs.len() > 10000 {
                    // 因为我们的 Ord 是倒序(最新的被视为最小放在前面)
                    // 所以树中的最后一个元素(最大的)就是最旧的日志
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
                // 直接使用 log_id 追踪展开状态，不再依赖索引
                if self.expanded_rows.contains(&log_id) {
                    self.expanded_rows.remove(&log_id);
                } else {
                    self.expanded_rows.insert(log_id);
                }
            }
            Message::CopyToClipboard(text) => {
                return iced::clipboard::write(text);
            }
        }
        Task::none()
    }

    fn apply_filters(&mut self) {
        self.filtered_logs.clear();
        
        // 将 filter_text 预先转为小写，避免在循环内部重复分配和转换，提升性能
        let filter_lower = self.filter_text.to_lowercase();

        // 遍历 BTreeSet (天然已经是按时间倒序排列的)
        for entry in &self.logs {
            let matches_text = filter_lower.is_empty()
                || entry.item.message.to_lowercase().contains(&filter_lower);

            let matches_level = match self.filter_level {
                None | Some(LogLevel::ALL) => true,
                Some(t) => entry.item.level == t.to_string(),
            };

            if matches_text && matches_level {
                // 因为使用了 Rc，这里的 clone 只是增加引用计数，极其轻量
                self.filtered_logs.push(entry.clone());
            }
        }
        // [移除原有排序逻辑] - 不再需要昂贵的 $O(N \log N)$ 排序
    }

    fn max_pages(&self) -> usize {
        self.filtered_logs.len().saturating_sub(1) / self.items_per_page
    }

    pub fn view(&self) -> Element<'_, Message> {
        let total_logs = self.logs.len();
        let filtered_count = self.filtered_logs.len();

        let start = self.current_page * self.items_per_page;
        let end = (start + self.items_per_page).min(filtered_count);
        let page_entries = if filtered_count == 0 {
            &[]
        } else {
            &self.filtered_logs[start..end]
        };

        // Header Styling
        let header: Element<Message> = Container::new(
            Row::new()
                .push(Text::new("Timestamp").width(Length::FillPortion(2)))
                .push(Text::new("Level").width(Length::FillPortion(2)))
                .push(Text::new("Type").width(Length::FillPortion(3)))
                .push(Text::new("Message").width(Length::FillPortion(12)))
                .spacing(10)
                .padding(8)
                .align_y(alignment::Vertical::Center),
        )
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.5, 0.5, 0.5, 0.2,
            ))),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into();

        // Table rows
        let table_rows: Vec<Element<Message>> = page_entries
            .iter()
            .enumerate()
            .map(|(local_idx, entry)| {
                let log = &entry.item;
                let log_id = entry.id;
                let is_expanded = self.expanded_rows.contains(&log_id);

                // Level colors
                let level_color = match log.level.to_lowercase().as_str() {
                    "error" => iced::Color::from_rgb(1.0, 0.4, 0.4),
                    "warning" => iced::Color::from_rgb(1.0, 0.8, 0.2),
                    "info" => iced::Color::from_rgb(0.4, 0.8, 1.0),
                    "debug" => iced::Color::from_rgb(0.6, 0.6, 0.6),
                    _ => iced::Color::WHITE,
                };

                // 【功能1】截断过长的 Message
                let display_message = truncate_text(&log.message, 100);

                let main_row = Row::new()
                    .push(
                        Text::new(&log.timestamp)
                            .size(14)
                            .width(Length::FillPortion(2)),
                    )
                    .push(
                        Text::new(&log.level)
                            .color(level_color)
                            .size(14)
                            .width(Length::FillPortion(2)),
                    )
                    .push(
                        Text::new(&log.r#type)
                            .size(14)
                            .width(Length::FillPortion(3)),
                    )
                    .push(
                        Text::new(display_message)
                            .size(14)
                            .width(Length::FillPortion(12)),
                    )
                    .spacing(10)
                    .padding(8)
                    .align_y(alignment::Vertical::Center);

                let clickable_row = button(main_row)
                    .on_press(Message::ToggleExpand(log_id))
                    .padding(0)
                    .width(Length::Fill)
                    .style(move |theme: &iced::Theme, status| {
                        let base_bg = if is_expanded {
                            iced::Color::from_rgba(0.5, 0.5, 0.5, 0.15)
                        } else if local_idx % 2 == 0 {
                            iced::Color::from_rgba(0.5, 0.5, 0.5, 0.05)
                        } else {
                            iced::Color::TRANSPARENT
                        };

                        let bg = match status {
                            button::Status::Hovered => iced::Color::from_rgba(0.5, 0.5, 0.5, 0.2),
                            button::Status::Pressed => iced::Color::from_rgba(0.5, 0.5, 0.5, 0.3),
                            _ => base_bg,
                        };

                        button::Style {
                            background: Some(iced::Background::Color(bg)),
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            text_color: theme.palette().text,
                            ..Default::default()
                        }
                    });

                if is_expanded {
                    let full_content = if log.details.trim().is_empty() {
                        log.message.clone()
                    } else {
                        format!("{}\n\n{}", log.message, log.details)
                    };

                    let copy_btn = button(Text::new("Copy").size(12))
                        .padding([4, 12])
                        .on_press(Message::CopyToClipboard(full_content.clone()));

                    let header_row = Row::new()
                        .push(Text::new("Full Log").size(13).width(Length::Shrink))
                        .push(Text::new("").width(Length::Fill))
                        .push(copy_btn)
                        .padding(Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 0.0 })
                        .align_y(alignment::Vertical::Center);

                    let details_row = Container::new(
                        Column::new()
                            .push(header_row)
                            .push(Text::new(full_content).size(13).width(Length::Fill))
                            .spacing(8)
                    )
                    .width(Length::Fill)
                    .padding(12)
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(
                            0.0, 0.0, 0.0, 0.2,
                        ))),
                        border: iced::Border {
                            radius: 4.0.into(),
                            color: iced::Color::from_rgba(0.5, 0.5, 0.5, 0.3),
                            width: 1.0,
                        },
                        ..Default::default()
                    });

                    Column::new()
                        .push(clickable_row)
                        .push(details_row)
                        .spacing(2)
                        .width(Length::Fill)
                        .into()
                } else {
                    clickable_row.into()
                }
            })
            .collect();

        let table: Element<Message> = Column::new()
            .push(header)
            .extend(table_rows)
            .spacing(4)
            .width(Length::Fill)
            .into();

        // Filters UI
        let filter_input = text_input("Search by message...", &self.filter_text)
            .on_input(Message::FilterTextChanged)
            .padding(8)
            .width(Length::FillPortion(3));

        let filter_type = pick_list(LogLevel::all_levels(), self.filter_level.as_ref(), |t| {
            Message::FilterLevelChanged(Some(t))
        })
        .placeholder("Filter by type...")
        .padding(8)
        .width(Length::FillPortion(1));

        let filters: Element<Message> = Row::new()
            .push(filter_input)
            .push(filter_type)
            .spacing(15)
            .align_y(alignment::Vertical::Center)
            .into();

        // Pagination UI
        let mut prev_btn = button("◄ Previous").padding([8, 16]);
        if self.current_page > 0 {
            prev_btn = prev_btn.on_press(Message::PrevPage);
        }

        let mut next_btn = button("Next ►").padding([8, 16]);
        if self.current_page < self.max_pages() {
            next_btn = next_btn.on_press(Message::NextPage);
        }

        let page_info = Text::new(format!(
            "Page {} of {}   |   Total Logs: {}   |   Filtered: {}",
            if filtered_count == 0 {
                0
            } else {
                self.current_page + 1
            },
            self.max_pages() + 1,
            total_logs,
            filtered_count
        ))
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Center);

        let pagination: Element<Message> = Row::new()
            .push(prev_btn)
            .push(page_info)
            .push(next_btn)
            .spacing(10)
            .align_y(alignment::Vertical::Center)
            .into();

        // Overall layout
        Column::new()
            .push(filters)
            .push(scrollable(Container::new(table).width(Length::Fill)).height(Length::Fill))
            .push(pagination)
            .spacing(20)
            .padding(20)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(watch_logs_stream)
    }
}