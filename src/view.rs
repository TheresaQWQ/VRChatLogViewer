use crate::log_entry::LogEntry;
use crate::log_entry::truncate_text;
use crate::message::{LogLevel, Message};
use iced::widget::{
    Column, Container, Row, Text, button, container, pick_list, scrollable, text_input,
};
use iced::{Element, Length, Padding, alignment};
use std::collections::HashSet;

// 日志主界面
pub fn view_logs<'a>(
    filtered_logs: &'a [LogEntry],
    filter_text: &'a str,
    filter_level: &'a Option<LogLevel>,
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
                    .padding(Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .align_y(alignment::Vertical::Center);

                let details_row = Container::new(
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

    // 顶部过滤器和设置按钮
    let filter_input = text_input("Search by message...", filter_text)
        .on_input(Message::FilterTextChanged)
        .padding(8)
        .width(Length::FillPortion(3));

    let filter_type = pick_list(LogLevel::all_levels(), filter_level.as_ref(), |t| {
        Message::FilterLevelChanged(Some(t))
    })
    .placeholder("Filter by type...")
    .padding(8)
    .width(Length::FillPortion(1));

    let settings_btn = button("⚙ Settings")
        .on_press(Message::OpenSettings)
        .padding(8);

    let filters: Element<Message> = Row::new()
        .push(filter_input)
        .push(filter_type)
        .push(settings_btn)
        .spacing(15)
        .align_y(alignment::Vertical::Center)
        .into();

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

    let pagination: Element<Message> = Row::new()
        .push(prev_btn)
        .push(page_info)
        .push(next_btn)
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into();

    Column::new()
        .push(filters)
        .push(scrollable(Container::new(table).width(Length::Fill)).height(Length::Fill))
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
