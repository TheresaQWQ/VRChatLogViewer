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

pub struct LogParser {
    pub offset: u64,
    fd: fs::File,
}

// 辅助函数：安全地移除 Unity 富文本标签，保留常规文本和类似 <T> 的泛型文本
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

        Self {
            offset: 0,
            fd,
        }
    }

    pub async fn parse(&mut self) -> Result<Vec<LogItem>, String> {
        let mut buffer: Vec<u8> = vec![];
        let mut buf = [0; 1024];

        match self.fd.seek(SeekFrom::Start(self.offset)).await {
            Ok(_) => {}
            Err(e) => {
                return Err(format!("Error seeking file: {}", e));
            }
        }

        // println!("Seeking from offset: {}", self.offset);

        loop {
            let n = self.fd.read(&mut buf).await;
            match n {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&buf[..n]);
                    self.offset += n as u64;
                }
                Err(e) => {
                    return Err(format!("Error reading file: {}", e));
                }
            }
        }

        let lines = buffer.split(|b| *b == b'\n');

        // println!("Parsing {} lines...", lines.clone().count());

        let mut current_log = LogItem {
            timestamp: "".to_string(),
            level: "".to_string(),
            r#type: "".to_string(),
            message: "".to_string(),
            details: "".to_string(),
        };

        let mut logs: Vec<LogItem> = vec![];

        for line in lines {
            let line = String::from_utf8_lossy(line);
            let line = line.to_string();

            if line.is_empty() {
                continue;
            }

            let is_new_log = line.len() >= 19
                && line.as_bytes().get(4) == Some(&b'.')
                && line.as_bytes().get(10) == Some(&b' ');

            if is_new_log {
                if !current_log.timestamp.is_empty() {
                    // self.logs.push_front(current_log);
                    logs.push(current_log);

                    current_log = LogItem {
                        timestamp: "".to_string(),
                        level: "".to_string(),
                        r#type: "".to_string(),
                        message: "".to_string(),
                        details: "".to_string(),
                    };
                }

                let timestamp = line[0..19].to_string();
                let rest = line[19..].trim();

                if let Some((level, msg)) = rest.split_once("- ") {
                    current_log.timestamp = timestamp;
                    current_log.level = level.trim().to_string();

                    if msg.trim_start().starts_with('[') {
                        if let Some(end_idx) = msg.find(']') {
                            current_log.r#type = strip_unity_tags(msg.trim_start()[1..end_idx-1].trim());
                            current_log.message = msg[end_idx + 1..].trim_start().to_string();
                        } else {
                            current_log.r#type = "NO_TYPE".to_string();
                            current_log.message = msg.to_string();
                        }
                    } else {
                        current_log.r#type = "NO_TYPE".to_string();
                        current_log.message = msg.trim_end().to_string();
                    }
                }
            } else {
                if !current_log.timestamp.is_empty() {
                    let line = line.trim_end();
                    if current_log.message.trim().is_empty() {
                        current_log.message = line.to_owned();
                    } else {
                        current_log.details =
                            format!("{}\n{}", current_log.details, line).to_string();
                    }
                }
            }
        }

        if !current_log.timestamp.is_empty() {
            logs.push(current_log);
        }

        let reversed_logs = logs.into_iter().rev().collect::<Vec<_>>();

        Ok(reversed_logs)
    }
}
