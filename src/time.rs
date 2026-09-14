use chrono::{DateTime, Local, TimeZone, Utc};
use serde_json::Value;

pub fn parse_to_epoch(val: &Value) -> Option<i64> {
    if let Some(n) = val.as_i64() {
        return Some(n);
    }
    if let Some(s) = val.as_str() {
        if let Ok(n) = s.parse::<i64>() {
            return Some(n);
        }
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp());
        }
        if let Ok(dt) = s.parse::<DateTime<Utc>>() {
            return Some(dt.timestamp());
        }
    }
    None
}

pub fn format_epoch_time(epoch: i64, style: &str) -> String {
    if epoch == 0 {
        return String::new();
    }
    let dt = match Local.timestamp_opt(epoch, 0) {
        chrono::LocalResult::Single(t) => t,
        _ => return String::new(),
    };

    let s = match style {
        "time" => dt.format("%-l:%M%P").to_string(),
        "datetime" => dt.format("%b %-d, %-l:%M%P").to_string(),
        _ => dt.format("%b %-d").to_string(),
    };
    s.trim_start().to_lowercase()
}

pub fn format_duration(dur_ms: i64) -> String {
    let elapsed = (dur_ms / 1000).max(0);
    if elapsed >= 3600 {
        format!("{}h{}m", elapsed / 3600, (elapsed % 3600) / 60)
    } else if elapsed >= 60 {
        format!("{}m", elapsed / 60)
    } else {
        format!("{}s", elapsed)
    }
}
