//! The calendars: one folder per calendar under `~/.local/share/filigrane/calendar/<slug>/`, with its name, its colour and
//! whether it is shown in `calendar.conf`. The colour is the name of an accent tone of the theme (`sable`, `sauge`…; `default`
//! follows the accent of the interface), so that a calendar never brings a colour of its own into the interface.
//! The flat folder of the first version is moved into "Personal" at the first start.
use std::{fs, path::PathBuf};

use crate::cli;

pub const DEFAULT: &str = "personal";
/// Tones a calendar can take (names of `[theme.accents]` in theme.toml)
pub const TONES: [&str; 7] = ["default", "voile", "ardoise", "sauge", "sable", "cendre", "brume"];

#[derive(Clone, Debug, PartialEq)]
pub struct Calendar {
    pub slug: String,
    pub name: String,
    pub color: String,
    pub hidden: bool,
    /// A subscription: the link of an iCalendar file that is fetched now and then. Its events are read-only.
    pub url: Option<String>,
    /// Minutes between two fetches of a subscription
    pub refresh: u32,
}

pub fn dir_of(slug: &str) -> PathBuf {
    cli::store().join(slug)
}

fn valid_slug(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn read_meta(slug: &str) -> Calendar {
    let text = fs::read_to_string(dir_of(slug).join("calendar.conf")).unwrap_or_default();
    let get = |k: &str| text.lines().find_map(|l| l.split_once('=').filter(|(a, _)| a.trim() == k).map(|(_, v)| v.trim().to_string()));
    Calendar {
        slug: slug.to_string(),
        name: get("name").filter(|n| !n.is_empty()).unwrap_or_else(|| if slug == DEFAULT { crate::i18n::tr("Personal").to_string() } else { slug.to_string() }),
        color: get("color").filter(|c| TONES.contains(&c.as_str())).unwrap_or_else(|| "default".into()),
        hidden: get("hidden").is_some_and(|h| h == "true"),
        url: get("url").filter(|u| !u.is_empty()),
        refresh: get("refresh").and_then(|r| r.parse().ok()).filter(|r| *r >= 5).unwrap_or(60),
    }
}

fn write_meta(c: &Calendar) -> Result<(), String> {
    fs::create_dir_all(dir_of(&c.slug)).map_err(|e| e.to_string())?;
    let mut text = format!("name = {}\ncolor = {}\nhidden = {}\n", c.name.replace('\n', " "), c.color, c.hidden);
    if let Some(u) = &c.url {
        text += &format!("url = {}\nrefresh = {}\n", u.replace(['\n', '\r'], ""), c.refresh);
    }
    fs::write(dir_of(&c.slug).join("calendar.conf"), text).map_err(|e| e.to_string())
}

/// Moves the `.ics` files of the flat folder of the first version into the default calendar
fn migrate() {
    let Ok(rd) = fs::read_dir(cli::store()) else { return };
    let flat: Vec<PathBuf> = rd.flatten().map(|f| f.path()).filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "ics")).collect();
    if flat.is_empty() {
        return;
    }
    let target = dir_of(DEFAULT);
    let _ = fs::create_dir_all(&target);
    if !target.join("calendar.conf").exists() {
        let _ = write_meta(&Calendar { slug: DEFAULT.into(), name: "Personal".into(), color: "default".into(), hidden: false, url: None, refresh: 60 });
    }
    for p in flat {
        if let Some(name) = p.file_name() {
            let _ = fs::rename(&p, target.join(name));
        }
    }
}

/// Every calendar, the default one first; the default one exists even when its folder does not yet
pub fn list() -> Vec<Calendar> {
    migrate();
    let mut slugs: Vec<String> = fs::read_dir(cli::store()).into_iter().flatten().flatten().filter(|f| f.path().is_dir()).filter_map(|f| f.file_name().to_str().map(str::to_string)).filter(|s| valid_slug(s)).collect();
    if !slugs.iter().any(|s| s == DEFAULT) {
        slugs.push(DEFAULT.into());
    }
    slugs.sort_by_key(|s| (s != DEFAULT, s.clone()));
    slugs.iter().map(|s| read_meta(s)).collect()
}

/// A calendar by its folder name or its name (any case)
pub fn find(key: &str) -> Option<Calendar> {
    let key = key.trim().to_lowercase();
    list().into_iter().find(|c| c.slug == key || c.name.to_lowercase() == key)
}

fn slug_of(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "calendar".into() } else { s }
}

pub fn create(name: &str, color: &str) -> Result<Calendar, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(crate::i18n::tr("the name is empty").to_string());
    }
    if find(name).is_some() {
        return Err(crate::i18n::trf("a calendar named %1 exists", &[&name]));
    }
    let base = slug_of(name);
    let mut slug = base.clone();
    let mut n = 2;
    while dir_of(&slug).exists() {
        slug = format!("{base}-{n}");
        n += 1;
    }
    let color = if TONES.contains(&color) { color } else { TONES[1 + list().len() % (TONES.len() - 1)] };
    let c = Calendar { slug, name: name.to_string(), color: color.to_string(), hidden: false, url: None, refresh: 60 };
    write_meta(&c)?;
    Ok(c)
}

/// Changes one field of a calendar: `name`, `color` or `hidden`
pub fn update(slug: &str, key: &str, value: &str) -> Result<(), String> {
    if !valid_slug(slug) {
        return Err(format!("not a calendar: {slug}"));
    }
    let mut c = read_meta(slug);
    match key {
        "name" => {
            let v = value.trim();
            if v.is_empty() {
                return Err(crate::i18n::tr("the name is empty").to_string());
            }
            if list().iter().any(|o| o.slug != slug && o.name.to_lowercase() == v.to_lowercase()) {
                return Err(crate::i18n::trf("a calendar named %1 exists", &[&v]));
            }
            c.name = v.to_string();
        }
        "color" if TONES.contains(&value) => c.color = value.to_string(),
        "color" => return Err(format!("color: {}", TONES.join(", "))),
        "hidden" => c.hidden = value == "true",
        "refresh" => c.refresh = value.parse::<u32>().ok().filter(|r| *r >= 5).ok_or("refresh: minutes, at least 5")?,
        _ => return Err(format!("unknown field: {key}")),
    }
    write_meta(&c)
}

/// Deletes a calendar and its events: the folder goes to the trash (the default calendar cannot be deleted)
pub fn remove(slug: &str) -> Result<(), String> {
    if slug == DEFAULT || !valid_slug(slug) {
        return Err(crate::i18n::tr("the default calendar cannot be deleted").to_string());
    }
    let dir = dir_of(slug);
    if !dir.is_dir() {
        return Err(format!("not a calendar: {slug}"));
    }
    let ok = std::process::Command::new("gio").arg("trash").arg(&dir).status().is_ok_and(|s| s.success());
    if ok { Ok(()) } else { Err(crate::i18n::tr("the trash is not available").to_string()) }
}

/// A subscription: `https://…` or `webcal://…` (a link is a secret: it is never printed or logged)
pub fn subscribe(name: &str, url: &str, color: &str, refresh: Option<u32>) -> Result<Calendar, String> {
    let url = url.trim();
    let url = url.strip_prefix("webcal://").map_or_else(|| url.to_string(), |r| format!("https://{r}"));
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.contains(char::is_whitespace) {
        return Err(crate::i18n::tr("a subscription needs an https:// or webcal:// link").to_string());
    }
    let mut c = create(name, color)?;
    c.url = Some(url);
    c.refresh = refresh.filter(|r| *r >= 5).unwrap_or(60);
    write_meta(&c)?;
    Ok(c)
}

/// The calendar of a folder (slug), when it is a subscription
pub fn subscription(slug: &str) -> Option<Calendar> {
    if !valid_slug(slug) {
        return None;
    }
    Some(read_meta(slug)).filter(|c| c.url.is_some())
}

pub fn json() -> String {
    serde_json::Value::Array(
        list()
            .iter()
            .map(|c| {
                let st = crate::subscribe::state(&c.slug);
                serde_json::json!({"slug": c.slug, "name": c.name, "color": c.color, "hidden": c.hidden, "subscribed": c.url.is_some(),
                    "fetched": st.fetched, "error": st.error})
            })
            .collect(),
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_files_move_into_personal_and_calendars_have_their_own_folder() {
        let _turn = cli::STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-cals-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the environment of this process is only read by the other tests through the same variable
        unsafe { std::env::set_var("XDG_DATA_HOME", &tmp) };
        fs::create_dir_all(cli::store()).unwrap();
        fs::write(cli::store().join("abc.ics"), "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n").unwrap();
        let all = list();
        assert_eq!(all.len(), 1);
        assert!(dir_of(DEFAULT).join("abc.ics").exists() && !cli::store().join("abc.ics").exists());
        let work = create("Work & Co", "sauge").unwrap();
        assert_eq!((work.slug.as_str(), work.color.as_str()), ("work-co", "sauge"));
        assert!(create("work & co", "").is_err());
        update("work-co", "hidden", "true").unwrap();
        assert!(find("WORK & CO").unwrap().hidden);
        assert!(update("work-co", "color", "pink").is_err());
        assert!(remove(DEFAULT).is_err());
        let _ = fs::remove_dir_all(&tmp);
    }
}
