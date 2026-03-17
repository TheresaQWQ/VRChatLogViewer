use crate::vrc;
use std::rc::Rc;

// 辅助函数：安全地截断过长的字符串
pub fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

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
        let cmp = other.item.timestamp.cmp(&self.item.timestamp);
        if cmp == std::cmp::Ordering::Equal {
            other.id.cmp(&self.id)
        } else {
            cmp
        }
    }
}
