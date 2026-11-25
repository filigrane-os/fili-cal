//! What the window shows, for whoever asks (`fili-context`, Mod+Y: "this event", "next week" mean something to the assistant). The
//! file `$XDG_RUNTIME_DIR/fili/context/<pid>.json` is written, atomically, on each change of the period or of the selected event
//! and removed on exit (docs/api.md of the distribution). Nothing runs in the background: it is read on demand.
use std::{fs, path::PathBuf};

use serde_json::{Value, json};

use crate::i18n;

fn path() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("fili/context").join(format!("{}.json", std::process::id())))
}

/// `state` holds what the window knows: view, from, to, day, events (the ones shown: id, title, start, end, place, calendar),
/// selected (the event being edited, or null)
pub fn publish(state: &str) {
    let (Some(p), Ok(Value::Object(mut s))) = (path(), serde_json::from_str::<Value>(state)) else { return };
    s.insert("app".into(), json!("Agenda"));
    s.insert("pid".into(), json!(std::process::id()));
    s.insert(
        "about".into(),
        json!(i18n::tr("Agenda shows the events and reminders of the user: `view` is day, week, month, year or list, `from` and `to` (excluded) the days shown, `day` the day the user is on, `events` the occurrences shown (without their notes) and `selected` the event being edited. Read and write events with `fili-cal`, not with the window.")),
    );
    // the file stays small: at most 60 events
    if let Some(Value::Array(a)) = s.get_mut("events") {
        a.truncate(60);
    }
    let text = Value::Object(s).to_string();
    if text.len() > 60_000 {
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let tmp = p.with_extension("json.new");
    if fs::write(&tmp, text).is_ok() {
        let _ = fs::rename(&tmp, &p);
    }
}

pub fn clear() {
    if let Some(p) = path() {
        let _ = fs::remove_file(p);
    }
}
