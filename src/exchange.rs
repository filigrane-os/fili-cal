//! Import and export of iCalendar files: `fili-cal import FILE|LINK`, `fili-cal export`, the Import and Export dialogs of the
//! window, a file dropped on it, and the file or `webcal:` link that the system hands over when Agenda is the handler of
//! `text/calendar` (an invitation opened from Facteur). An import is one step of the journal (`fili-cal undo`).
use std::fs;

use serde_json::{Value, json};

use crate::{calendars, cli, ics, subscribe};

/// The text of a source: a path, a `file://` URL, or an `https://` / `webcal://` link (fetched once, not subscribed)
pub fn read_source(source: &str) -> Result<String, String> {
    let s = source.trim();
    if s.starts_with("webcal://") || s.starts_with("webcals://") || s.starts_with("https://") || s.starts_with("http://") {
        let url = s.replacen("webcals://", "https://", 1).replacen("webcal://", "https://", 1);
        return subscribe::download(&url);
    }
    let path = s.strip_prefix("file://").map_or_else(|| s.to_string(), percent_decode);
    let path = if let Some(rest) = path.strip_prefix("~/") { format!("{}/{rest}", cli::home()) } else { path };
    fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
}

use crate::ics::percent_decode;

/// What an import would add, for the preview: {ok, count, events: [{title, start, allDay, place, every}] (the first 40)}
pub fn preview(source: &str) -> Value {
    let text = match read_source(source) {
        Ok(t) => t,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    if !text.contains("BEGIN:VCALENDAR") && !text.contains("BEGIN:VEVENT") {
        return json!({"ok": false, "error": crate::i18n::tr("this is not an iCalendar file")});
    }
    let events = ics::parse_ics(&text);
    if events.is_empty() {
        return json!({"ok": false, "error": crate::i18n::tr("there is no event in this file")});
    }
    let rows: Vec<Value> = events
        .iter()
        .take(40)
        .map(|e| json!({"title": e.summary, "start": e.display_start().format("%Y-%m-%dT%H:%M").to_string(), "allDay": e.all_day, "place": e.place, "every": e.rule.as_ref().map_or("", |r| r.freq.name())}))
        .collect();
    json!({"ok": true, "count": events.len(), "events": rows, "invite": crate::invites::describe(&text)})
}

/// Adds the events of a file to a calendar (an event with the same UID in that calendar is replaced). (added, replaced)
pub fn import(source: &str, calendar: &str) -> Result<(usize, usize), String> {
    let cal = if calendar.is_empty() { calendars::find(calendars::DEFAULT) } else { calendars::find(calendar) }.ok_or_else(|| format!("no calendar named {calendar}"))?;
    if cal.url.is_some() {
        return Err(crate::i18n::trf("%1 is a subscription: its events cannot be changed", &[&cal.name]));
    }
    let events = ics::parse_ics(&read_source(source)?);
    if events.is_empty() {
        return Err(crate::i18n::tr("there is no event in this file").to_string());
    }
    let (mut added, mut replaced) = (0, 0);
    for e in events {
        let id = cli::short_hash(&format!("{}/{}", cal.slug, e.uid));
        if cli::path_in(&cal.slug, &id).is_ok_and(|p| p.exists()) {
            replaced += 1;
        } else {
            added += 1;
        }
        cli::save(&id, &ics::Event { calendar: cal.slug.clone(), ..e })?;
    }
    crate::undo::commit(&format!("import {added} events"));
    let _ = cli::tick();
    Ok((added, replaced))
}

/// One iCalendar text with the events of a calendar (None: every calendar, subscriptions included)
pub fn export(calendar: Option<&str>) -> Result<String, String> {
    let slug = match calendar.filter(|c| !c.is_empty() && *c != "all") {
        Some(c) => Some(calendars::find(c).ok_or_else(|| format!("no calendar named {c}"))?.slug),
        None => None,
    };
    let mut out = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Filigrane//fili-cal//EN\r\n");
    for (_, e) in cli::load_all() {
        if slug.as_deref().is_some_and(|s| (if e.calendar.is_empty() { calendars::DEFAULT } else { &e.calendar }) != s) {
            continue;
        }
        let t = ics::to_ics(&e);
        let a = t.find("BEGIN:VEVENT").unwrap_or(0);
        let b = t.rfind("END:VCALENDAR").unwrap_or(t.len());
        out.push_str(&t[a..b]);
    }
    out.push_str("END:VCALENDAR\r\n");
    Ok(out)
}

/// Writes an export to a file (`~` is the home folder; a folder gets `agenda.ics` in it)
pub fn export_to(calendar: Option<&str>, path: &str) -> Result<String, String> {
    export_to_text(&export(calendar)?, path)
}

pub fn export_to_text(text: &str, path: &str) -> Result<String, String> {
    let p = path.trim();
    let mut p = if let Some(rest) = p.strip_prefix("~/") { format!("{}/{rest}", cli::home()) } else { p.to_string() };
    if std::path::Path::new(&p).is_dir() {
        p = format!("{}/agenda.ics", p.trim_end_matches('/'));
    }
    if p.is_empty() {
        return Err(crate::i18n::tr("the file name is empty").to_string());
    }
    fs::write(&p, text).map_err(|e| format!("{p}: {e}"))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_previewed_imported_replaced_and_exported() {
        let _turn = cli::STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-exchange-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the tests that set the store take turns (STORE_LOCK)
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.join("data"));
            std::env::set_var("XDG_STATE_HOME", tmp.join("state"));
        }
        fs::create_dir_all(&tmp).unwrap();
        let file = tmp.join("invite one.ics");
        fs::write(&file, "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:inv@x\r\nDTSTART:20261210T150000\r\nSUMMARY:Lunch\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n").unwrap();
        let src = file.display().to_string();
        let v = preview(&format!("file://{}", src.replace(' ', "%20")));
        assert_eq!((v["ok"].as_bool(), v["count"].as_u64(), v["events"][0]["title"].as_str()), (Some(true), Some(1), Some("Lunch")));
        assert_eq!(preview(&tmp.join("nope.ics").display().to_string())["ok"].as_bool(), Some(false));
        fs::write(tmp.join("page.html"), "<html></html>").unwrap();
        assert_eq!(preview(&tmp.join("page.html").display().to_string())["ok"].as_bool(), Some(false));
        let work = calendars::create("Work", "").unwrap();
        assert_eq!(import(&src, "work").unwrap(), (1, 0));
        // the import is one step: undone, the event is gone; done again, it is back
        crate::undo::undo().unwrap();
        assert_eq!(cli::load_all().len(), 0);
        crate::undo::redo().unwrap();
        assert_eq!(import(&src, "work").unwrap(), (0, 1));
        assert_eq!(cli::load_all().len(), 1);
        let text = export(Some("work")).unwrap();
        assert!(text.contains("SUMMARY:Lunch") && text.starts_with("BEGIN:VCALENDAR") && text.ends_with("END:VCALENDAR\r\n"));
        assert!(!export(Some("personal")).unwrap().contains("Lunch"));
        // a subscription takes nothing
        let sub = calendars::subscribe("Feed", "https://example.org/f.ics", "", None).unwrap();
        assert!(import(&src, &sub.slug).is_err());
        let _ = work;
        assert_eq!(export_to(Some("work"), &tmp.display().to_string()).unwrap(), format!("{}/agenda.ics", tmp.display()));
        let _ = fs::remove_dir_all(&tmp);
    }
}
