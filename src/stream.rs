use crate::message::Message;
use crate::vrc;
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn watch_logs_stream() -> impl iced::futures::Stream<Item = Message> + Send + 'static {
    let (sender, receiver) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut watch_rx = vrc::watch::watch_logs().await;
        let mut parsers: HashMap<PathBuf, Arc<Mutex<vrc::parser::LogParser>>> = HashMap::new();

        // println!("Watching for new log files...");

        loop {
            if let Some(path) = watch_rx.recv().await {
                let mut sender_clone = sender.clone();
                let parser_lock = if let Some(p) = parsers.get(&path) {
                    p.clone()
                } else {
                    let parser =
                        vrc::parser::LogParser::new(path.to_string_lossy().to_string()).await;
                    let p = Arc::new(Mutex::new(parser));
                    parsers.insert(path.clone(), p.clone());
                    p
                };

                tokio::spawn(async move {
                    let mut parser = parser_lock.lock().await;

                    match parser.parse().await {
                        Ok(logs) => {
                            for log in logs {
                                let _ = sender_clone.send(Message::NewLog(log.clone())).await;
                            }
                        }
                        Err(err) => {
                            eprintln!("Error parsing log: {}", err);
                        }
                    }
                });
            }
        }
    });

    receiver
}
