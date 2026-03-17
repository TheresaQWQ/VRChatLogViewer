use crate::message::Message;
use crate::vrc;
use chrono::{Duration as ChronoDuration, Local, NaiveDateTime};
use iced::futures::SinkExt;
use iced::futures::Stream;
use iced::futures::channel::mpsc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

pub fn watch_logs_stream() -> impl Stream<Item = Message> + Send + 'static {
    let (sender, receiver) = mpsc::channel(100);

    tokio::spawn(async move {
        let log_dir = match crate::vrc::path::get_vrchat_app_data_location() {
            Some(p) => p,
            None => {
                eprintln!("Error: VRChat app data location not found.");
                return;
            }
        };

        let mut parsers: HashMap<PathBuf, Arc<Mutex<vrc::parser::LogParser>>> = HashMap::new();

        const PREFIX: &str = "output_log_";
        const SUFFIX: &str = ".txt";
        const TIME_LEN: usize = 19; // YYYY-MM-DD_HH-MM-SS
        const MIN_LEN: usize = PREFIX.len() + TIME_LEN + SUFFIX.len();
        const TIME_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

        loop {
            let now = Local::now().naive_local();
            let cutoff = now - ChronoDuration::days(2);

            let mut entries = match fs::read_dir(&log_dir).await {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Failed to read log directory: {}", err);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // 收集有效文件及其时间
            let mut valid_files_with_time: Vec<(PathBuf, NaiveDateTime)> = Vec::new();

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                if !filename.starts_with(PREFIX) || !filename.ends_with(SUFFIX) {
                    continue;
                }

                if filename.len() < MIN_LEN {
                    continue;
                }

                let time_str = &filename[PREFIX.len()..PREFIX.len() + TIME_LEN];

                let Ok(file_time) = NaiveDateTime::parse_from_str(time_str, TIME_FORMAT) else {
                    continue;
                };

                if file_time < cutoff {
                    continue;
                }

                valid_files_with_time.push((path, file_time));
            }

            // 按时间倒序排序
            valid_files_with_time.sort_by_key(|(_, time)| std::cmp::Reverse(*time));

            for (path, _) in &valid_files_with_time {
                let parser_lock = if let Some(p) = parsers.get(path) {
                    p.clone()
                } else {
                    let parser =
                        vrc::parser::LogParser::new(path.to_string_lossy().to_string()).await;
                    let p = Arc::new(Mutex::new(parser));
                    parsers.insert(path.clone(), p.clone());
                    p
                };

                let mut sender_clone = sender.clone();
                let mut parser = parser_lock.lock().await;
                match parser.parse().await {
                    Ok(logs) => {
                        for log in logs {
                            let _ = sender_clone.send(Message::NewLog(log.clone())).await;
                        }
                    }
                    Err(err) => {
                        eprintln!("Error parsing log {}: {}", path.display(), err);
                    }
                }
            }

            // 清理内存：保留最近 2 天的文件解析器
            let valid_paths: HashSet<_> = valid_files_with_time.iter().map(|(p, _)| p.clone()).collect();
            parsers.retain(|k, _| valid_paths.contains(k));

            sleep(Duration::from_secs(1)).await;
        }
    });

    receiver
}