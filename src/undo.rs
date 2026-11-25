//! Undo and redo of the changes made to the store. One request is one transaction: a command of `fili-cal`, or one action of
//! the window (a save, a delete, a move). Each transaction keeps the text of every file it touched before and after, in a
//! journal under `~/.local/state/filigrane/fili-cal/journal/`, so `fili-cal undo` (what Lia says after a change) and Ctrl+Z
//! restore the files exactly. Like `fili-tx` for the files of the user.
use std::{fs, path::PathBuf, sync::Mutex};

use serde_json::{Value, json};

use crate::cli;

/// How many transactions are kept
const KEEP: u64 = 200;

struct Change {
    id: String,
    before: Option<String>,
    after: Option<String>,
}

static PENDING: Mutex<Vec<Change>> = Mutex::new(Vec::new());

fn pending() -> std::sync::MutexGuard<'static, Vec<Change>> {
    PENDING.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn dir() -> PathBuf {
    cli::state().join("fili-cal").join("journal")
}

fn cursor() -> u64 {
    fs::read_to_string(dir().join("cursor")).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0)
}

fn tx_path(n: u64) -> PathBuf {
    dir().join(format!("{n:08}.json"))
}

/// A file of the store changed (None = it did not exist, or does not any more)
pub fn record(id: &str, before: Option<String>, after: Option<String>) {
    if before != after {
        pending().push(Change { id: id.to_string(), before, after });
    }
}

/// Closes the transaction: everything recorded since the last commit becomes one step of the journal (nothing when empty).
/// A new step ends the redo line.
pub fn commit(label: &str) {
    let changes: Vec<Change> = std::mem::take(&mut *pending());
    if changes.is_empty() {
        return;
    }
    let (d, n) = (dir(), cursor());
    let _ = fs::create_dir_all(&d);
    let mut gone = n + 1;
    while fs::remove_file(tx_path(gone)).is_ok() {
        gone += 1;
    }
    let rows: Vec<Value> = changes.iter().map(|c| json!({"id": c.id, "before": c.before, "after": c.after})).collect();
    let tx = json!({"label": label, "changes": rows});
    if fs::write(tx_path(n + 1), tx.to_string()).is_ok() {
        let _ = fs::write(d.join("cursor"), (n + 1).to_string());
        if n + 1 > KEEP {
            let _ = fs::remove_file(tx_path(n + 1 - KEEP));
        }
    }
}

fn put(id: &str, text: &Option<String>) -> Result<(), String> {
    let p = cli::path_of(id)?;
    match text {
        Some(t) => {
            fs::create_dir_all(cli::store()).map_err(|e| e.to_string())?;
            fs::write(&p, t).map_err(|e| e.to_string())
        }
        None => match fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

fn read(n: u64) -> Result<Value, String> {
    let text = fs::read_to_string(tx_path(n)).map_err(|_| crate::i18n::tr("nothing to redo").to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn apply(tx: &Value, key: &str, reverse: bool) -> Result<(), String> {
    let mut list: Vec<&Value> = tx["changes"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    if reverse {
        list.reverse();
    }
    for c in list {
        put(c["id"].as_str().unwrap_or(""), &c[key].as_str().map(str::to_string))?;
    }
    Ok(())
}

/// Takes the last step back; returns its label
pub fn undo() -> Result<String, String> {
    let n = cursor();
    if n == 0 {
        return Err(crate::i18n::tr("nothing to undo").to_string());
    }
    let tx = read(n).map_err(|_| crate::i18n::tr("nothing to undo").to_string())?;
    apply(&tx, "before", true)?;
    let _ = fs::write(dir().join("cursor"), (n - 1).to_string());
    let _ = cli::tick();
    Ok(tx["label"].as_str().unwrap_or("").to_string())
}

/// Does again the step that was undone; returns its label
pub fn redo() -> Result<String, String> {
    let n = cursor() + 1;
    let tx = read(n).map_err(|_| crate::i18n::tr("nothing to redo").to_string())?;
    apply(&tx, "after", false)?;
    let _ = fs::write(dir().join("cursor"), n.to_string());
    let _ = cli::tick();
    Ok(tx["label"].as_str().unwrap_or("").to_string())
}

/// (can undo, can redo, label of the step an undo would take back)
pub fn state() -> (bool, bool, String) {
    let n = cursor();
    let label = if n > 0 { read(n).ok().and_then(|t| t["label"].as_str().map(str::to_string)).unwrap_or_default() } else { String::new() };
    (n > 0 && tx_path(n).exists(), tx_path(n + 1).exists(), label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_is_undone_and_done_again_and_a_new_step_ends_the_redo_line() {
        let tmp = std::env::temp_dir().join(format!("fili-cal-undo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: only this test of the program sets these variables
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.join("data"));
            std::env::set_var("XDG_STATE_HOME", tmp.join("state"));
        }
        let e = |t: &str| crate::ics::Event { uid: "u@x".into(), summary: t.into(), start: chrono::NaiveDate::from_ymd_opt(2026, 11, 25).unwrap().and_hms_opt(9, 0, 0).unwrap(), ..Default::default() };
        cli::save("a1", &e("one")).unwrap();
        commit("add one");
        cli::save("a1", &e("two")).unwrap();
        commit("rename");
        assert_eq!(state(), (true, false, "rename".into()));
        assert_eq!(undo().unwrap(), "rename");
        assert_eq!(cli::load("a1").unwrap().summary, "one");
        assert_eq!(undo().unwrap(), "add one");
        assert!(cli::load("a1").is_err());
        assert!(undo().is_err());
        assert_eq!(redo().unwrap(), "add one");
        assert_eq!(cli::load("a1").unwrap().summary, "one");
        // a new step while one is undone: the old redo is gone
        cli::save("a1", &e("three")).unwrap();
        commit("again");
        assert!(redo().is_err());
        assert_eq!(state(), (true, false, "again".into()));
        let _ = fs::remove_dir_all(&tmp);
    }
}
