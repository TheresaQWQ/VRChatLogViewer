use crate::log_entry::truncate_text;
use crate::log_entry::LogEntry;
use crate::message::{LogLevel, Message};
use iced::widget::{
    button, container, pick_list, scrollable, text_input, tooltip, Column, Container, Row, Text,
};
use iced::{alignment, Element, Length, Padding};
use std::collections::HashSet;

// 日志主界面
pub fn view_logs<'a>(
    filtered_logs: &'a [LogEntry],
    filter_text: &'a str,
    filter_level: &'a Option<LogLevel>,
    filter_regex_enabled: bool,
    filter_regex_error: Option<&'a str>,
    current_page: usize,
    items_per_page: usize,
    expanded_rows: &'a HashSet<usize>,
    total_logs: usize,
) -> Element<'a, Message> {
    let filtered_count = filtered_logs.len();

    let start = current_page * items_per_page;
    let end = (start + items_per_page).min(filtered_count);
    let page_entries = if filtered_count == 0 {
        &[]
    } else {
        &filtered_logs[start..end]
    };

    let table_header = Container::new(
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
    });

    let table_rows: Vec<Element<Message>> = page_entries
        .iter()
        .enumerate()
        .map(|(local_idx, entry)| {
            let log = &entry.item;
            let log_id = entry.id;
            let is_expanded = expanded_rows.contains(&log_id);

            let level_color = match log.level.to_lowercase().as_str() {
                "error" => iced::Color::from_rgb(1.0, 0.4, 0.4),
                "warning" => iced::Color::from_rgb(1.0, 0.8, 0.2),
                "info" => iced::Color::from_rgb(0.4, 0.8, 1.0),
                "debug" => iced::Color::from_rgb(0.6, 0.6, 0.6),
                _ => iced::Color::WHITE,
            };

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
                let full_content = log.raw.clone();

                let copy_btn = button(Text::new("Copy").size(12))
                    .padding([4, 12])
                    .on_press(Message::CopyToClipboard(full_content.clone()));

                let header_row = Row::new()
                    .push(Text::new("Full Log").size(13).width(Length::Shrink))
                    .push(Text::new("").width(Length::Fill))
                    .push(copy_btn)
                    .padding(Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_y(alignment::Vertical::Center);

                let full_log_row = Container::new(
                    Column::new()
                        .push(header_row)
                        .push(Text::new(full_content).size(13).width(Length::Fill))
                        .spacing(8),
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
                    .push(full_log_row)
                    .spacing(2)
                    .width(Length::Fill)
                    .into()
            } else {
                clickable_row.into()
            }
        })
        .collect();

    let table_rows_scrollable = scrollable(Column::new().extend(table_rows));

    let table = Column::new()
        .push(table_header)
        .push(table_rows_scrollable)
        .spacing(4);

    // 顶部过滤器和设置按钮
    let filter_placeholder = if filter_regex_enabled {
        "Search by regex..."
    } else {
        "Search by message..."
    };

    let has_regex_error = filter_regex_error.is_some();

    let filter_input = text_input(filter_placeholder, filter_text)
        .on_input(Message::FilterTextChanged)
        .padding([8, 10])
        .style(|theme, status| {
            let base = iced::widget::text_input::default(theme, status);

            iced::widget::text_input::Style {
                background: iced::Background::Color(iced::Color::TRANSPARENT),
                border: iced::Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: iced::Color::TRANSPARENT,
                },
                ..base
            }
        });

    let search_input = filter_input.width(Length::Fill);

    let regex_control_label = if has_regex_error { "!" } else { ".*" };
    let regex_tooltip_text = if let Some(error) = filter_regex_error {
        error
    } else if filter_regex_enabled {
        "Regex search enabled"
    } else {
        "Regex search disabled"
    };

    let regex_control_text_color = if filter_regex_enabled || has_regex_error {
        iced::Color::WHITE
    } else {
        iced::Color::from_rgb(0.78, 0.78, 0.78)
    };

    let regex_control_content = Container::new(
        Text::new(regex_control_label)
            .size(14)
            .color(regex_control_text_color),
    )
    .width(Length::Fixed(40.0))
    .height(Length::Fixed(30.0))
    .center_x(Length::Fixed(40.0))
    .center_y(Length::Fixed(30.0));

    let regex_control_button = button(regex_control_content)
        .padding(0)
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(30.0))
        .on_press(Message::FilterRegexToggled(!filter_regex_enabled))
        .style(move |_theme, status| {
            let base_bg = if has_regex_error {
                iced::Color::from_rgb(0.78, 0.18, 0.18)
            } else if filter_regex_enabled {
                iced::Color::from_rgb(0.18, 0.42, 0.78)
            } else {
                iced::Color::from_rgba(0.5, 0.5, 0.5, 0.16)
            };

            let background = match status {
                button::Status::Hovered => {
                    if has_regex_error {
                        iced::Color::from_rgb(0.88, 0.24, 0.24)
                    } else if filter_regex_enabled {
                        iced::Color::from_rgb(0.24, 0.5, 0.9)
                    } else {
                        iced::Color::from_rgba(0.5, 0.5, 0.5, 0.26)
                    }
                }
                button::Status::Pressed => {
                    if has_regex_error {
                        iced::Color::from_rgb(0.62, 0.12, 0.12)
                    } else if filter_regex_enabled {
                        iced::Color::from_rgb(0.12, 0.3, 0.62)
                    } else {
                        iced::Color::from_rgba(0.5, 0.5, 0.5, 0.34)
                    }
                }
                _ => base_bg,
            };

            button::Style {
                background: Some(iced::Background::Color(background)),
                border: iced::Border {
                    radius: 4.0.into(),
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                    width: 1.0,
                },
                text_color: regex_control_text_color,
                ..Default::default()
            }
        });

    let regex_tooltip = Container::new(
        Text::new(regex_tooltip_text)
            .size(12)
            .width(Length::Fixed(360.0))
            .color(iced::Color::WHITE),
    )
    .padding(10)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(
            0.08, 0.08, 0.08, 0.95,
        ))),
        border: iced::Border {
            radius: 4.0.into(),
            color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.18),
            width: 1.0,
        },
        ..Default::default()
    });

    let regex_control: Element<Message> = tooltip(
        regex_control_button,
        regex_tooltip,
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into();

    let search_box = Container::new(
        Row::new()
            .push(search_input)
            .push(regex_control)
            .spacing(0)
            .align_y(alignment::Vertical::Center),
    )
    .width(Length::FillPortion(3))
    .padding(Padding {
        top: 2.0,
        right: 3.0,
        bottom: 2.0,
        left: 0.0,
    })
    .style(move |_theme| {
        let border_color = if has_regex_error {
            iced::Color::from_rgb(0.78, 0.18, 0.18)
        } else if filter_regex_enabled {
            iced::Color::from_rgb(0.18, 0.42, 0.78)
        } else {
            iced::Color::from_rgba(0.5, 0.5, 0.5, 0.42)
        };

        container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.5, 0.5, 0.5, 0.08,
            ))),
            border: iced::Border {
                radius: 4.0.into(),
                color: border_color,
                width: 1.0,
            },
            ..Default::default()
        }
    });

    let filter_type = pick_list(LogLevel::all_levels(), filter_level.as_ref(), |t| {
        Message::FilterLevelChanged(Some(t))
    })
    .placeholder("Filter by type...")
    .padding(8);

    let settings_btn = button("⚙ Settings")
        .on_press(Message::OpenSettings)
        .padding(8);

    let filters = Row::new()
        .push(search_box)
        .push(filter_type.width(Length::FillPortion(1)))
        .push(settings_btn)
        .spacing(15)
        .align_y(alignment::Vertical::Center);

    // 底部翻页
    let max_pages = filtered_logs.len().saturating_sub(1) / items_per_page;

    let mut prev_btn = button("◄ Previous").padding([8, 16]);
    if current_page > 0 {
        prev_btn = prev_btn.on_press(Message::PrevPage);
    }

    let mut next_btn = button("Next ►").padding([8, 16]);
    if current_page < max_pages {
        next_btn = next_btn.on_press(Message::NextPage);
    }

    let page_info = Text::new(format!(
        "Page {} of {}   |   Total Logs: {}   |   Filtered: {}",
        if filtered_count == 0 {
            0
        } else {
            current_page + 1
        },
        max_pages + 1,
        total_logs,
        filtered_count
    ))
    .width(Length::Fill)
    .align_x(alignment::Horizontal::Center);

    let pagination = Row::new()
        .push(prev_btn)
        .push(page_info)
        .push(next_btn)
        .spacing(10)
        .align_y(alignment::Vertical::Center);

    Column::new()
        .push(filters)
        .push(table.width(Length::Fill).height(Length::Fill))
        .push(pagination)
        .spacing(20)
        .padding(20)
        .into()
}

// 设置界面
pub fn view_settings<'a>(
    temp_file_count: &'a str,
    temp_initial_logs: &'a str,
    temp_max_cached: &'a str,
) -> Element<'a, Message> {
    let title = Text::new("Settings").size(28);

    let file_count_row = Row::new()
        .push(
            Text::new("Log files to watch:")
                .width(Length::Fixed(200.0))
                .align_y(alignment::Vertical::Center),
        )
        .push(
            text_input("e.g. 1", temp_file_count)
                .on_input(Message::SettingsFileCountChanged)
                .padding(8),
        )
        .spacing(20)
        .align_y(alignment::Vertical::Center);

    let initial_logs_row = Row::new()
        .push(
            Text::new("Initial logs per file:")
                .width(Length::Fixed(200.0))
                .align_y(alignment::Vertical::Center),
        )
        .push(
            text_input("e.g. 2000", temp_initial_logs)
                .on_input(Message::SettingsInitialLogsChanged)
                .padding(8),
        )
        .spacing(20)
        .align_y(alignment::Vertical::Center);

    let max_cached_row = Row::new()
        .push(
            Text::new("Max cached logs in memory:")
                .width(Length::Fixed(200.0))
                .align_y(alignment::Vertical::Center),
        )
        .push(
            text_input("e.g. 10000", temp_max_cached)
                .on_input(Message::SettingsMaxCachedChanged)
                .padding(8),
        )
        .spacing(20)
        .align_y(alignment::Vertical::Center);

    let save_btn = button("Save & Apply")
        .on_press(Message::SaveSettings)
        .padding([10, 20]);

    let cancel_btn = button("Cancel")
        .on_press(Message::CloseSettings)
        .padding([10, 20]);

    let button_row = Row::new().push(save_btn).push(cancel_btn).spacing(15);

    let content = Column::new()
        .push(title)
        .push(file_count_row)
        .push(initial_logs_row)
        .push(max_cached_row)
        .push(button_row)
        .spacing(25)
        .max_width(500.0);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
