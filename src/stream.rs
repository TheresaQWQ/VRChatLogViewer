use crate::message::Message;
use crate::vrc;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use iced::futures::Stream;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use tokio::fs;
use chrono::{Local, Duration as ChronoDuration, NaiveDateTime};

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
            
            let mut valid_files: HashSet<PathBuf> = HashSet::new();

            let mut entries = match fs::read_dir(&log_dir).await {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Failed to read log directory: {}", err);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                
                // 必须是文件
                if !path.is_file() {
                    continue;
                }

                let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                // 基础格式检查
                if !filename.starts_with(PREFIX) || !filename.ends_with(SUFFIX) {
                    continue;
                }

                // 长度检查防止 panic
                if filename.len() < MIN_LEN {
                    continue;
                }

                // 提取时间字符串
                let time_str = &filename[PREFIX.len()..PREFIX.len() + TIME_LEN];

                // 解析时间
                let Ok(file_time) = NaiveDateTime::parse_from_str(time_str, TIME_FORMAT) else {
                    continue;
                };

                // 4. 时间过滤：只处理最近 2 天的文件
                if file_time < cutoff {
                    continue;
                }

                // 标记为有效文件
                valid_files.insert(path.clone());

                // 5. 获取或创建解析器
                let parser_lock = if let Some(p) = parsers.get(&path) {
                    p.clone()
                } else {
                    // 新发现的有效文件，创建解析器
                    let parser = vrc::parser::LogParser::new(path.to_string_lossy().to_string()).await;
                    let p = Arc::new(Mutex::new(parser));
                    parsers.insert(path.clone(), p.clone());
                    p
                };

                let mut sender_clone = sender.clone();
                tokio::spawn(async move {
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
                });
            }

            // 7. 清理内存：移除超过 2 天或已删除文件的解析器
            parsers.retain(|k, _| valid_files.contains(k));

            // 8. 轮询间隔
            sleep(Duration::from_secs(1)).await;
        }
    });

    receiver
}