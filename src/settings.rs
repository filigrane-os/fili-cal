//! Settings of Agenda: the `[agenda]` section of `~/.config/filigrane/fili-cal.toml` (optional, absent = the defaults below).
//!
//! ```toml
//! [agenda]
//! first_day = "monday"      # monday … sunday; default: the first day of the language
//! clock = 24                # 24 or 12 hours
//! default_length = "1h"     # length of a new event
//! default_alert = "10m"     # alert of a new event, "none" for no alert
//! work_week = false         # hide Saturday and Sunday in the week and month views
//! week_numbers = false      # the number of the week in the month view
//! travel_time = "0m"        # an extra alert this long before an event that has a place ("20m")
//! first_hour = 7            # the hour the day and week views scroll to when today is not shown
//! place_suggestions = true   # suggest real places as the Place field is typed (the button next to the field sets it)
//! place_search = "https://photon.komoot.io/api/"  # the Photon server asked for them (OpenStreetMap); "none" to turn off
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
    let _ = fs::write(&p, "[agenda]\n# first_day = \"monday\"     # monday ... sunday; default: the first day of the language\n# clock = 24             # 24 or 12 hours\n# default_length = \"1h\"  # length of a new event\n# default_alert = \"10m\"  # alert of a new timed event, \"none\" for no alert\n# work_week = false      # hide Saturday and Sunday in the week and month views\n# week_numbers = false   # the number of the week in the month view\n# travel_time = \"20m\"  # an extra alert this long before an event that has a place\n# first_hour = 7         # the hour the day and week views scroll to when today is not shown\n# all_day_alert_hour = 9 # the hour the alert of a whole-day event rings\n# place_suggestions = true  # suggest real places as the Place field is typed (sends that text to the server below)\n# place_search = \"https://photon.komoot.io/api/\"  # the Photon server asked for them (OpenStreetMap), \"none\" to turn off\n");
}

/// The Photon server that suggests places as they are typed (komoot's public one by default: free, open source, OpenStreetMap
/// data); "none" or "" turns the suggestions off. Only the text typed in the Place field is sent.
pub fn place_server() -> String {
    match value("place_search") {
        Some(v) if v == "none" || v.is_empty() => String::new(),
        Some(v) => v,
        None => "https://photon.komoot.io/api/".to_string(),
    }
}

/// Whether the Place field suggests places: on by default, turned off by the button next to the field (`place_suggestions`)
pub fn place_suggestions() -> bool {
    value("place_suggestions").is_none_or(|v| v != "false")
}

/// The server the Place field asks while typing, empty when the suggestions are off
pub fn place_search() -> String {
    if place_suggestions() { place_server() } else { String::new() }
}

/// Writes `key = value` in the `[agenda]` section of fili-cal.toml (value as TOML: `true`, `"1h"`): replaces the key, or its
/// commented-out line, or adds it at the end of the section; the rest of the file and its comments stay as they are
pub fn set(key: &str, value: &str) -> std::io::Result<()> {
    ensure_file();
    let p = theme::path();
    let text = fs::read_to_string(&p).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let new = format!("{key} = {value}");
    let (mut section, mut set_at, mut commented, mut end) = (None::<bool>, None, None, None);
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix('[').and_then(|r| r.split(']').next()) {
            if section == Some(true) && end.is_none() {
                end = Some(i);
            }
            section = Some(name.trim() == "agenda");
            continue;
        }
        if section != Some(true) {
            continue;
        }
        let k = |s: &str| s.split_once('=').map(|(k, _)| k.trim().to_string());
        if k(t.split('#').next().unwrap_or("")).as_deref() == Some(key) {
            set_at = Some(i);
        } else if let Some(c) = t.strip_prefix('#') && k(c).as_deref() == Some(key) && commented.is_none() {
            commented = Some(i);
        }
    }
    match (set_at, commented) {
        (Some(i), _) | (None, Some(i)) => lines[i] = new,
        (None, None) if section.is_some() && lines.iter().any(|l| l.trim() == "[agenda]") => {
            let at = end.unwrap_or(lines.len());
            lines.insert(at, new);
        }
        _ => {
            lines.push("[agenda]".to_string());
            lines.push(new);
        }
    }
    fs::write(&p, lines.join("\n") + "\n")
}

/// Time to get to an event that has a place, in seconds (an extra alert that long before it; 0 = none, the default)
pub fn travel_time() -> i64 {
    value("travel_time").and_then(|v| when::parse_duration(&v).ok()).filter(|s| *s > 0).unwrap_or(0)
}

/// The hour an alert of a whole-day event rings (default 9)
pub fn all_day_hour() -> u32 {
    value("all_day_alert_hour").and_then(|h| h.parse::<u32>().ok()).filter(|h| *h < 24).unwrap_or(9)
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
        "workWeek": value("work_week").is_some_and(|w| w == "true"),
        "weekNumbers": value("week_numbers").is_some_and(|w| w == "true"),
        "firstHour": value("first_hour").and_then(|h| h.parse::<u32>().ok()).filter(|h| *h < 24).unwrap_or(7),
        "defaultLength": default_length() / 60,
        "defaultAlert": default_alert().unwrap_or(-1),
        "placeSearch": place_search(),
        "placeSuggestions": place_suggestions(),
        "placeServer": place_server(),
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
        assert_eq!(place_search(), "https://photon.komoot.io/api/");
        fs::create_dir_all(tmp.join("filigrane")).unwrap();
        fs::write(theme::path(), "[theme]\nfirst_day = \"friday\"\n[agenda]\nfirst_day = \"monday\" # comment\nclock = 12\ndefault_length = \"45m\"\ndefault_alert = \"none\"\nfirst_hour = 8\n").unwrap();
        assert_eq!((default_length(), default_alert()), (2700, None));
        fs::write(theme::path(), "[agenda]\nplace_search = \"none\"\n").unwrap();
        assert_eq!(place_search(), "");
        // The button of the Place field: the key is written in [agenda], over its commented-out line, the rest kept
        fs::write(theme::path(), "[theme]\naccent = \"blue\"\n[agenda]\n# place_suggestions = true  # note\nclock = 12\n[other]\nx = 1\n").unwrap();
        set("place_suggestions", "false").unwrap();
        assert!(!place_suggestions());
        assert_eq!(place_search(), "");
        assert_eq!(place_server(), "https://photon.komoot.io/api/");
        set("place_suggestions", "true").unwrap();
        assert!(place_suggestions());
        assert_eq!(fs::read_to_string(theme::path()).unwrap(), "[theme]\naccent = \"blue\"\n[agenda]\nplace_suggestions = true\nclock = 12\n[other]\nx = 1\n");
        fs::write(theme::path(), "[theme]\naccent = \"blue\"\n").unwrap();
        set("place_suggestions", "false").unwrap();
        assert_eq!(fs::read_to_string(theme::path()).unwrap(), "[theme]\naccent = \"blue\"\n[agenda]\nplace_suggestions = false\n");
        fs::write(theme::path(), "[theme]\nfirst_day = \"friday\"\n[agenda]\nfirst_day = \"monday\" # comment\nclock = 12\ndefault_length = \"45m\"\ndefault_alert = \"none\"\nfirst_hour = 8\n").unwrap();
        let v: serde_json::Value = serde_json::from_str(&json()).unwrap();
        assert_eq!((v["firstDay"].as_i64(), v["clock12"].as_bool(), v["firstHour"].as_i64(), v["defaultAlert"].as_i64()), (Some(1), Some(true), Some(8), Some(-1)));
        let _ = fs::remove_dir_all(&tmp);
    }
}
