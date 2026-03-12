use std::path::PathBuf;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

pub async fn watch_logs() -> mpsc::Receiver<PathBuf> {
    let (tx, rx) = mpsc::channel::<PathBuf>(100);

    let _ = tokio::spawn(async move {
        let path = match super::path::get_vrchat_app_data_location() {
            Some(p) => p,
            None => {
                eprintln!("Error: VRChat app data location not found.");
                return;
            }
        };

        loop {
            let mut files = match fs::read_dir(path.clone()).await {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("Error reading directory: {}", e);
                    continue;
                }
            };

            while let Ok(Some(entry)) = files.next_entry().await {
                let file = entry.path();
                // output_log_2026-03-13_04-41-07.txt
                if !file.is_file() {
                    continue;
                }

                let Some(filename) = file.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };

                if !filename.starts_with("output_log_") || !filename.ends_with(".txt") {
                    continue;
                }

                // println!("Found new log file: {}", file.display());
                let _ = tx.send(file).await;

                time::sleep(Duration::from_secs(1)).await;
            }
        }
    });

    // println!("Watching for new log files...");

    rx
}
