use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub file_count: usize,
    pub initial_logs_per_file: usize,
    pub max_cached_logs: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            file_count: 2,               // 默认读取 1 个最新的文件
            initial_logs_per_file: 5000, // 默认初始读取 5000 条
            max_cached_logs: 10000,      // 默认最大缓存 10000 条
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string("config.json") {
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write("config.json", content);
        }
    }
}
