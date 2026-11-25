//! Settings of Agenda: the `[agenda]` section of `~/.config/filigrane/fili-cal.toml` (optional, absent = the defaults below).
//!
//! ```toml
//! [agenda]
//! first_day = "monday"      # monday … sunday; default: the first day of the language
//! clock = 24                # 24 or 12 hours
//! default_length = "1h"     # length of a new event
//! default_alert = "10m"     # alert of a new event, "none" for no alert
//! first_hour = 7            # the hour the day and week views scroll to when today is not shown
//! ```
use std::fs;

use crate::{theme, when};

fn value(key: &str) -> Option<String> {
    let text = fs::read_to_string(theme::path()).ok()?;
    let mut in_section = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if let Some(name) = line.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            in_section = name.trim() == "agenda";
        } else if in_section && let Some((k, v)) = line.split_once('=') && k.trim() == key {
            return Some(v.trim().trim_matches('"').trim().to_string());
        }
    }
    None
}

/// The file Settings… opens: created with the keys commented out the first time, so that there is something to edit
pub fn ensure_file() {
    let p = theme::path();
    if p.exists() {
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(&p, "[agenda]\n# first_day = \"monday\"     # monday ... sunday; default: the first day of the language\n# clock = 24             # 24 or 12 hours\n# default_length = \"1h\"  # length of a new event\n# default_alert = \"10m\"  # alert of a new timed event, \"none\" for no alert\n# first_hour = 7         # the hour the day and week views scroll to when today is not shown\n");
}

/// Length of a new event in seconds (default one hour)
pub fn default_length() -> i64 {
    value("default_length").and_then(|v| when::parse_duration(&v).ok()).filter(|s| *s > 0).unwrap_or(3600)
}

/// Alert of a new timed event, seconds before the start (default 10 minutes); None = no alert
pub fn default_alert() -> Option<i64> {
    match value("default_alert") {
        Some(v) if v.eq_ignore_ascii_case("none") => None,
        Some(v) => Some(when::parse_duration(&v).unwrap_or(600)),
        None => Some(600),
    }
}

pub fn default_alarms() -> Vec<i64> {
    default_alert().into_iter().collect()
}

/// The settings the window reads, as JSON: first_day (0 Sunday … 6 Saturday, -1 = the language), clock12, first_hour,
/// default_length (minutes), default_alert (seconds, -1 none)
pub fn json() -> String {
    let first_day = value("first_day").map_or(-1, |d| match d.to_lowercase().as_str() {
        "sunday" => 0,
        "monday" => 1,
        "tuesday" => 2,
        "wednesday" => 3,
        "thursday" => 4,
        "friday" => 5,
        "saturday" => 6,
        _ => -1,
    });
    serde_json::json!({
        "firstDay": first_day,
        "clock12": value("clock").is_some_and(|c| c == "12"),
        "firstHour": value("first_hour").and_then(|h| h.parse::<u32>().ok()).filter(|h| *h < 24).unwrap_or(7),
        "defaultLength": default_length() / 60,
        "defaultAlert": default_alert().unwrap_or(-1),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_without_a_file_and_values_from_it() {
        let tmp = std::env::temp_dir().join(format!("fili-cal-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: only this test sets this variable
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp) };
        assert_eq!((default_length(), default_alert()), (3600, Some(600)));
        fs::create_dir_all(tmp.join("filigrane")).unwrap();
        fs::write(theme::path(), "[theme]\nfirst_day = \"friday\"\n[agenda]\nfirst_day = \"monday\" # comment\nclock = 12\ndefault_length = \"45m\"\ndefault_alert = \"none\"\nfirst_hour = 8\n").unwrap();
        assert_eq!((default_length(), default_alert()), (2700, None));
        let v: serde_json::Value = serde_json::from_str(&json()).unwrap();
        assert_eq!((v["firstDay"].as_i64(), v["clock12"].as_bool(), v["firstHour"].as_i64(), v["defaultAlert"].as_i64()), (Some(1), Some(true), Some(8), Some(-1)));
        let _ = fs::remove_dir_all(&tmp);
    }
}
