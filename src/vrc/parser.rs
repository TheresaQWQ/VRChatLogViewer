use std::cmp::Ordering;
use std::io::SeekFrom;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt},
};

#[derive(Debug, Clone)]
pub struct LogItem {
    pub timestamp: String,
    pub level: String,
    pub r#type: String,
    pub message: String,
    pub details: String,
}

impl Ord for LogItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other.timestamp.cmp(&self.timestamp)
    }
}

impl PartialOrd for LogItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LogItem {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.level == other.level
            && self.message == other.message
    }
}

impl Eq for LogItem {}

pub struct LogParser {
    pub offset: u64,
    fd: fs::File,
}

// 移除 Unity 富文本标签
fn strip_unity_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut buffer = String::new();
            let mut is_tag = false;
            while let Some(&next_c) = chars.peek() {
                if next_c == '>' {
                    is_tag = true;
                    chars.next();
                    break;
                }
                if next_c == '<' {
                    break;
                }
                buffer.push(next_c);
                chars.next();
            }

            if is_tag {
                let tag_name = buffer
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/');
                let is_known = matches!(
                    tag_name.to_lowercase().as_str(),
                    "color"
                        | "b"
                        | "i"
                        | "size"
                        | "material"
                        | "quad"
                        | "a"
                        | "mark"
                        | "u"
                        | "s"
                        | "sup"
                        | "sub"
                        | "voffset"
                        | "sprite"
                        | "allcaps"
                        | "smallcaps"
                        | "upper"
                        | "lower"
                        | "line-height"
                        | "align"
                        | "width"
                        | "margin"
                        | "indent"
                        | "nobr"
                );
                if is_known {
                    continue;
                }
            }

            result.push('<');
            result.push_str(&buffer);
            if is_tag {
                result.push('>');
            }
        } else {
            result.push(c);
        }
    }
    result
}

impl LogParser {
    pub async fn new(log_file: String) -> Self {
        let fd = fs::File::open(&log_file).await.unwrap();
        Self { offset: 0, fd }
    }

    pub async fn parse(&mut self) -> Result<Vec<LogItem>, String> {
        if self.offset > 0 {
            self.parse_incremental().await
        } else {
            self.parse_tail().await
        }
    }

    pub async fn parse_incremental(&mut self) -> Result<Vec<LogItem>, String> {
        let mut buf = [0u8; 1024];
        let mut buffer: Vec<u8> = Vec::new();

        self.fd
            .seek(SeekFrom::Start(self.offset))
            .await
            .map_err(|e| e.to_string())?;

        loop {
            let n = self.fd.read(&mut buf).await.map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&buf[..n]);
            self.offset += n as u64;
        }

        Self::parse_lines(&buffer)
    }

    pub async fn parse_tail(&mut self) -> Result<Vec<LogItem>, String> {
        const MAX_LOGS: usize = 5000;
        const BUF_SIZE: usize = 8192;

        let file_len = self.fd.metadata().await.map_err(|e| e.to_string())?.len();
        let mut pos = file_len;
        let mut logs: Vec<LogItem> = Vec::new();
        let mut leftover: Vec<u8> = Vec::new(); // 保存当前块开头可能残缺的行

        while pos > 0 && logs.len() < MAX_LOGS {
            let read_size = std::cmp::min(BUF_SIZE as u64, pos) as usize;
            pos -= read_size as u64;

            self.fd
                .seek(SeekFrom::Start(pos))
                .await
                .map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; read_size];
            self.fd
                .read_exact(&mut buf)
                .await
                .map_err(|e| e.to_string())?;

            // 将残余行加到本块前面
            buf.extend_from_slice(&leftover);
            let mut lines: Vec<&[u8]> = buf.split(|b| *b == b'\n').collect();

            // 本块开头可能不完整
            leftover = if !lines.is_empty() {
                lines.remove(0).to_vec()
            } else {
                Vec::new()
            };

            // 反序解析当前块
            for line in lines.into_iter().rev() {
                let line = String::from_utf8_lossy(line).to_string();
                if line.is_empty() {
                    continue;
                }

                if let Some(log_item) = Self::parse_line_to_log(&line) {
                    logs.push(log_item);
                    if logs.len() >= MAX_LOGS {
                        break;
                    }
                } else if let Some(last) = logs.last_mut() {
                    // 多行日志拼接
                    let line = line.trim_end();
                    if last.message.trim().is_empty() {
                        last.message = line.to_owned();
                    } else {
                        last.details = format!("{}\n{}", last.details, line);
                    }
                }
            }
        }

        self.offset = file_len;

        logs.reverse(); // 最新日志在前
        Ok(logs)
    }

    /// 辅助函数：解析整块字节
    fn parse_lines(buffer: &[u8]) -> Result<Vec<LogItem>, String> {
        let lines = buffer.split(|b| *b == b'\n');
        let mut logs: Vec<LogItem> = Vec::new();
        let mut current = LogItem {
            timestamp: "".to_string(),
            level: "".to_string(),
            r#type: "".to_string(),
            message: "".to_string(),
            details: "".to_string(),
        };

        for line in lines {
            let line = String::from_utf8_lossy(line).to_string();
            if line.is_empty() {
                continue;
            }

            if let Some(log_item) = Self::parse_line_to_log(&line) {
                if !current.timestamp.is_empty() {
                    logs.push(current);
                }
                current = log_item;
            } else if !current.timestamp.is_empty() {
                let line = line.trim_end();
                if current.message.trim().is_empty() {
                    current.message = line.to_owned();
                } else {
                    current.details = format!("{}\n{}", current.details, line);
                }
            }
        }

        if !current.timestamp.is_empty() {
            logs.push(current);
        }

        Ok(logs.into_iter().rev().collect())
    }

    fn parse_line_to_log(line: &str) -> Option<LogItem> {
        if line.len() < 19
            || line.as_bytes().get(4) != Some(&b'.')
            || line.as_bytes().get(10) != Some(&b' ')
        {
            return None;
        }

        let timestamp = line[0..19].to_string();
        let rest = line[19..].trim();

        if let Some((level, msg)) = rest.split_once("- ") {
            let mut log_item = LogItem {
                timestamp,
                level: level.trim().to_string(),
                r#type: "NO_TYPE".to_string(),
                message: "".to_string(),
                details: "".to_string(),
            };

            if msg.trim_start().starts_with('[') {
                if let Some(end_idx) = msg.find(']') {
                    log_item.r#type = strip_unity_tags(msg.trim_start()[1..end_idx - 1].trim());
                    log_item.message = msg[end_idx + 1..].trim_start().to_string();
                } else {
                    log_item.message = msg.to_string();
                }
            } else {
                log_item.message = msg.trim_end().to_string();
            }
            Some(log_item)
        } else {
            None
        }
    }
}
