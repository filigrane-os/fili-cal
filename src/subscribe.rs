//! Subscriptions: a calendar whose events come from the link of an iCalendar file (Proton's private link, a public holiday
//! calendar…). The file is fetched with `curl` (no TLS stack in this program), asking only for what changed (`ETag`,
//! `Last-Modified`), split into one file per event in the folder of the calendar, and the last good copy stays when the
//! network is down. The events are read-only. `fili-cal tick` starts the fetch in the background when one is due, so it never waits.
//! The link is a secret: it is given to curl on its standard input (not on the command line) and never printed.
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::{calendars, cli, ics};

#[derive(Default, Clone)]
pub struct State {
    pub etag: String,
    pub modified: String,
    /// Epoch of the last successful check (0 = never)
    pub fetched: i64,
    pub error: String,
}

fn state_path(slug: &str) -> PathBuf {
    calendars::dir_of(slug).join("subscription.json")
}

pub fn state(slug: &str) -> State {
    let v: Value = fs::read_to_string(state_path(slug)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let text = |k: &str| v[k].as_str().unwrap_or("").to_string();
    State { etag: text("etag"), modified: text("modified"), fetched: v["fetched"].as_i64().unwrap_or(0), error: text("error") }
}

fn save_state(slug: &str, s: &State) {
    let _ = fs::write(state_path(slug), json!({"etag": s.etag, "modified": s.modified, "fetched": s.fetched, "error": s.error}).to_string());
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Is a fetch due: never done, or older than the refresh time (a failed one counts as made 5 minutes before it is due)
pub fn due(c: &calendars::Calendar) -> bool {
    let st = state(&c.slug);
    c.url.is_some() && (st.fetched == 0 || now() - st.fetched >= i64::from(c.refresh) * 60)
}

/// The header value of a curl dump (`-D`), the last response (after redirects)
fn header(dump: &str, name: &str) -> String {
    dump.lines().rev().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_string())).unwrap_or_default()
}

/// What curl got: (HTTP status, headers, body)
fn fetch(url: &str, st: &State) -> Result<(u16, String, Vec<u8>), String> {
    let tmp = cli::state().join("fili-cal");
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let (body, head) = (tmp.join(format!("fetch-{}.body", std::process::id())), tmp.join(format!("fetch-{}.head", std::process::id())));
    let mut cfg = format!("url = \"{}\"\nsilent\nshow-error\nlocation\nmax-time = 40\nmax-filesize = 30000000\nproto = \"=https,http\"\noutput = \"{}\"\ndump-header = \"{}\"\nwrite-out = \"%{{http_code}}\"\n", url.replace('"', "%22"), body.display(), head.display());
    if !st.etag.is_empty() {
        cfg += &format!("header = \"If-None-Match: {}\"\n", st.etag.replace('"', "\\\""));
    }
    if !st.modified.is_empty() {
        cfg += &format!("header = \"If-Modified-Since: {}\"\n", st.modified);
    }
    let mut child = Command::new("curl").args(["--config", "-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|_| crate::i18n::tr("curl is not installed").to_string())?;
    child.stdin.take().ok_or("no pipe")?.write_all(cfg.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let code: u16 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0);
    let (bytes, dump) = (fs::read(&body).unwrap_or_default(), fs::read_to_string(&head).unwrap_or_default());
    let _ = (fs::remove_file(&body), fs::remove_file(&head));
    if !out.status.success() {
        // curl's message never holds the link (it was not on the command line), but cut it anyway
        let msg = String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("").replace(url, "…");
        return Err(msg);
    }
    Ok((code, dump, bytes))
}

/// Writes the events of a downloaded file as the files of the calendar: the ones that changed, and no more of the ones that
/// went away. Returns how many events there are.
pub fn store_events(slug: &str, text: &str) -> Result<usize, String> {
    if !text.trim_start().starts_with("BEGIN:VCALENDAR") {
        return Err(crate::i18n::tr("the link does not give a calendar").to_string());
    }
    let events = ics::parse_ics(text);
    let dir = calendars::dir_of(slug);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut keep = std::collections::HashSet::new();
    for e in events.iter().take(20000) {
        // the calendar is part of the id: the same event in a subscription and in a calendar of the user are two files
        let id = cli::short_hash(&format!("{slug}/{}", e.uid));
        let path = dir.join(format!("{id}.ics"));
        let body = ics::to_ics(e);
        // DTSTAMP changes at every write: compare without it
        let plain = |t: &str| t.lines().filter(|l| !l.starts_with("DTSTAMP:")).collect::<Vec<_>>().join("\n");
        if fs::read_to_string(&path).map_or(true, |old| plain(&old) != plain(&body)) {
            fs::write(&path, body).map_err(|e| e.to_string())?;
        }
        keep.insert(format!("{id}.ics"));
    }
    for f in fs::read_dir(&dir).into_iter().flatten().flatten() {
        let name = f.file_name().to_string_lossy().to_string();
        if name.ends_with(".ics") && !keep.contains(&name) {
            let _ = fs::remove_file(f.path());
        }
    }
    Ok(keep.len())
}

/// Fetches one subscription now. Ok(Some(n)) = n events stored, Ok(None) = nothing changed since the last time.
pub fn sync(c: &calendars::Calendar) -> Result<Option<usize>, String> {
    let url = c.url.as_deref().ok_or_else(|| crate::i18n::tr("this calendar is not a subscription").to_string())?;
    let mut st = state(&c.slug);
    let result = fetch(url, &st).and_then(|(code, dump, body)| match code {
        304 => Ok(None),
        200 => {
            let n = store_events(&c.slug, &String::from_utf8_lossy(&body))?;
            st.etag = header(&dump, "etag");
            st.modified = header(&dump, "last-modified");
            Ok(Some(n))
        }
        other => Err(crate::i18n::trf("the server answered %1", &[&other])),
    });
    match &result {
        Ok(_) => {
            st.fetched = now();
            st.error.clear();
        }
        Err(e) => {
            st.error.clone_from(e);
            // the time of the failure counts as an attempt: the next one is in 5 minutes
            st.fetched = now() - i64::from(c.refresh) * 60 + 300;
        }
    }
    save_state(&c.slug, &st);
    result
}

/// `fili-cal calendar sync [NAME]`: the one named (always), or every subscription that is due
pub fn sync_command(name: Option<&str>) -> Result<(), String> {
    let _lock = Lock::take().ok_or_else(|| crate::i18n::tr("a fetch is already running").to_string())?;
    let list: Vec<calendars::Calendar> = match name {
        Some(n) => vec![calendars::find(n).ok_or_else(|| format!("no calendar named {n}"))?],
        None => calendars::list().into_iter().filter(due).collect(),
    };
    let mut failed = Vec::new();
    for c in &list {
        match sync(c) {
            Ok(Some(n)) => println!("{}\t{n}", c.name),
            Ok(None) => println!("{}\tunchanged", c.name),
            Err(e) => {
                println!("{}\t{e}", c.name);
                failed.push(c.name.clone());
            }
        }
    }
    let _ = cli::tick();
    if failed.is_empty() { Ok(()) } else { Err(crate::i18n::trf("could not fetch: %1", &[&failed.join(", ")])) }
}

/// Starts a fetch of what is due in a process of its own, without waiting for it (what `tick` and the window do)
pub fn spawn_due() {
    if !calendars::list().iter().any(due) {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).args(["calendar", "sync"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

/// One fetch at a time: a file made when it starts, ignored when older than five minutes
struct Lock(PathBuf);
impl Lock {
    fn take() -> Option<Lock> {
        let p = cli::state().join("fili-cal").join("sync.lock");
        let _ = fs::create_dir_all(p.parent()?);
        if fs::metadata(&p).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|e| e.as_secs() > 300) {
            let _ = fs::remove_file(&p);
        }
        fs::OpenOptions::new().write(true).create_new(true).open(&p).ok().map(|_| Lock(p))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_downloaded_file_becomes_one_file_per_event_and_old_ones_go() {
        let _turn = cli::STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-sub-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the tests that set the store take turns (STORE_LOCK)
        unsafe { std::env::set_var("XDG_DATA_HOME", &tmp) };
        let c = calendars::subscribe("Holidays", "webcal://example.org/h.ics", "sauge", Some(120)).unwrap();
        assert_eq!((c.url.as_deref(), c.refresh), (Some("https://example.org/h.ics"), 120));
        assert!(calendars::subscribe("Bad", "ftp://x", "", None).is_err());
        let two = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261225\r\nSUMMARY:Christmas\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART;VALUE=DATE:20270101\r\nSUMMARY:New Year\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        assert_eq!(store_events(&c.slug, two).unwrap(), 2);
        let one = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261225\r\nSUMMARY:Christmas\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        assert_eq!(store_events(&c.slug, one).unwrap(), 1);
        let count = fs::read_dir(calendars::dir_of(&c.slug)).unwrap().flatten().filter(|f| f.path().extension().is_some_and(|e| e == "ics")).count();
        assert_eq!(count, 1);
        assert!(store_events(&c.slug, "<html>login</html>").is_err());
        // a subscription is due when never fetched
        assert!(due(&calendars::find("holidays").unwrap()));
        assert!(calendars::subscription(&c.slug).is_some() && calendars::subscription("personal").is_none());
        let _ = fs::remove_dir_all(&tmp);
    }
}
