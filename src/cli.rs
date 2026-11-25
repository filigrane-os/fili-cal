// Command line of fili-cal (the same program opens the window without arguments): the calendar and the reminders, one list. A reminder is an event with an alarm at its start, so there
// is a single thing to learn, to show in the bar and to ask the assistant for. No daemon: the events are iCalendar files
// (~/.local/share/filigrane/calendar/<id>.ics, readable by any calendar program) and `fili-cal tick`, run every minute by the
// fili-bar, shows the alarms that are due and rewrites the file the clock card reads.
//
// fili-cal add TITLE --at WHEN [--end WHEN | --for DURATION] [--place P] [--note N] [--alarm DURATION|none]...
//                    [--every day|week|month|year] [--interval N] [--count N] [--until WHEN]
//                                  add an event; prints its id. --reminder = alarm at the start (the default alarm of an
//                                  event is 10 minutes before; none for a whole day)
// fili-cal quick WHEN TITLE...     the same on one line, for a text field: "tomorrow 14:30 Dentist", "demain à 9h Appeler la banque",
//                                  "2026-12-24 Christmas Eve" (whole day), "+30m Take the bread out" (a reminder: alarm at the start)
// fili-cal edit ID [the options of add, and --title T]   change an event
// fili-cal rm ID [--on WHEN]        delete an event, or only one occurrence of a series
// fili-cal show ID                  one event
// fili-cal list [--from WHEN] [--to WHEN | --days N] [--tsv | --json]   occurrences (default: today and the next 30 days)
// fili-cal next [N]                 the next N occurrences (default 5)
// fili-cal tick                     alarms due since the last tick (notification), and the file of the clock card
// fili-cal import FILE.ics          add the events of an iCalendar file (the same event twice is replaced)
// fili-cal export [ID]              iCalendar text of every event, or one
// WHEN: +2h · 14:30 or 14h30 (the next one) · 2026-12-24 (whole day) · 2026-12-24 20:00 · tomorrow 09:00 · monday 18:00 (the
// next one; French names work too). DURATION: 30s 10m 2h 1d 1w 1h30m (a bare number is minutes).
// Environment: FILI_CAL_NOW=YYYY-MM-DDTHH:MM replaces the clock, FILI_CAL_NOTIFY the command that shows an alarm (default
// notify-send), both for the tests.
use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    process::{Command, Stdio},
};

use chrono::{Duration, NaiveDateTime, Timelike};
use crate::ics::{self, Event, Freq, Rule};
use crate::{calendars, dav, exchange, i18n, invites, subscribe, undo, when};
use serde_json::json;

/// The tests that point the store at a folder of their own take turns
#[cfg(test)]
pub static STORE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn home() -> String {
    env::var("HOME").unwrap_or_default()
}
pub fn dir(var: &str, fallback: &str, tail: &str) -> PathBuf {
    PathBuf::from(env::var(var).unwrap_or_else(|_| format!("{}/{fallback}", home()))).join(tail)
}
pub fn store() -> PathBuf {
    dir("XDG_DATA_HOME", ".local/share", "filigrane/calendar")
}
pub fn state() -> PathBuf {
    dir("XDG_STATE_HOME", ".local/state", "filigrane")
}

pub fn short_hash(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:08x}", h.finish() as u32)
}

pub fn new_id() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    short_hash(&format!("{nanos}{}", std::process::id()))
}

fn valid_id(id: &str) -> Result<(), String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("not an event id: {id}"));
    }
    Ok(())
}

/// The file of an event in a given calendar
pub fn path_in(calendar: &str, id: &str) -> Result<PathBuf, String> {
    valid_id(id)?;
    let cal = if calendar.is_empty() { calendars::DEFAULT } else { calendar };
    if !cal.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("not a calendar: {calendar}"));
    }
    Ok(calendars::dir_of(cal).join(format!("{id}.ics")))
}

/// The file of an event wherever it is (an id is unique across the calendars); a new event is in the default calendar
pub fn path_of(id: &str) -> Result<PathBuf, String> {
    valid_id(id)?;
    calendars::list().iter().map(|c| calendars::dir_of(&c.slug).join(format!("{id}.ics"))).find(|p| p.exists()).map_or_else(|| path_in(calendars::DEFAULT, id), Ok)
}

/// The calendar a file belongs to (its folder)
fn calendar_of(p: &std::path::Path) -> String {
    p.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).unwrap_or(calendars::DEFAULT).to_string()
}

/// What the files of the store gave: the events, and the files that could not be read (name, why)
pub struct Scan {
    pub events: Vec<(String, Event)>,
    pub unreadable: Vec<(String, String)>,
}

type Cached = (std::time::SystemTime, u64, Option<Event>);

/// Parsed files of this process, by name and modification time: `tick` (every minute) and the window (on each change) read
/// only the files that changed, so thousands of events stay cheap
fn cache() -> &'static std::sync::Mutex<std::collections::HashMap<PathBuf, Cached>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<PathBuf, Cached>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Reads the store. A file that cannot be read, or that holds no event, is reported instead of disappearing.
pub fn scan() -> Scan {
    let mut out = Scan { events: Vec::new(), unreadable: Vec::new() };
    let mut cache = cache().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut seen = std::collections::HashSet::new();
    for cal in calendars::list() {
        let Ok(rd) = fs::read_dir(calendars::dir_of(&cal.slug)) else { continue };
        for f in rd.flatten() {
            let p = f.path();
            if p.extension().is_none_or(|e| e != "ics") {
                continue;
            }
            let Some(id) = p.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
            let meta = fs::metadata(&p).ok();
            let stamp = (meta.as_ref().and_then(|m| m.modified().ok()).unwrap_or(std::time::UNIX_EPOCH), meta.as_ref().map_or(0, std::fs::Metadata::len));
            seen.insert(p.clone());
            let hit = cache.get(&p).filter(|c| (c.0, c.1) == stamp).cloned();
            let entry = hit.unwrap_or_else(|| {
                let parsed = fs::read_to_string(&p).ok().and_then(|text| ics::parse_ics(&text).into_iter().next()).map(|e| Event { calendar: cal.slug.clone(), ..e });
                (stamp.0, stamp.1, parsed)
            });
            match &entry.2 {
                Some(e) => out.events.push((id, e.clone())),
                None => out.unreadable.push((id, crate::i18n::tr("no event in it").to_string())),
            }
            cache.insert(p, entry);
        }
    }
    cache.retain(|k, _| seen.contains(k));
    out.events.sort_by_key(|(id, e)| (e.display_start(), id.clone()));
    out.unreadable.sort();
    out
}

/// (id, event) of every file of the store
pub fn load_all() -> Vec<(String, Event)> {
    scan().events
}

pub fn load(id: &str) -> Result<Event, String> {
    let p = path_of(id)?;
    let text = fs::read_to_string(&p).map_err(|_| format!("no such event: {id}"))?;
    ics::parse_ics(&text).into_iter().next().map(|e| Event { calendar: calendar_of(&p), ..e }).ok_or_else(|| format!("unreadable event: {id}"))
}

/// The events of a subscription come from its link: they cannot be changed here
fn read_only(calendar: &str, id: &str) -> Result<(), String> {
    if let Some(c) = calendars::subscription(calendar) {
        return Err(crate::i18n::trf("%1 is a subscription: its events cannot be changed", &[&c.name]));
    }
    // an event of an account that the server keeps as several events (a series with a changed occurrence)
    if calendars::account(calendar).is_some() && crate::dav::locked(&crate::dav::load(calendar), id) {
        return Err(crate::i18n::tr("this event is made of several parts on the server: change it in the application it comes from").to_string());
    }
    Ok(())
}

/// Writes an event in its calendar (a changed calendar moves the file)
pub fn save(id: &str, e: &Event) -> Result<(), String> {
    let p = path_in(&e.calendar, id)?;
    read_only(&calendar_of(&p), id)?;
    fs::create_dir_all(p.parent().ok_or("no folder")?).map_err(|x| x.to_string())?;
    let before = fs::read_to_string(&p).ok();
    let tmp = p.with_extension("ics.new");
    let text = ics::to_ics(e);
    fs::write(&tmp, &text).map_err(|x| x.to_string())?;
    fs::rename(&tmp, &p).map_err(|x| x.to_string())?;
    undo::record(&calendar_of(&p), id, before, Some(text));
    // the same id in another calendar: the event moved
    for c in calendars::list() {
        let q = calendars::dir_of(&c.slug).join(format!("{id}.ics"));
        // a subscription is never touched from here (its files are those of its link)
        if q != p && q.exists() && calendars::subscription(&c.slug).is_none() {
            let old = fs::read_to_string(&q).ok();
            fs::remove_file(&q).map_err(|x| x.to_string())?;
            undo::record(&c.slug, id, old, None);
        }
    }
    Ok(())
}

/// Deletes the file of an event (the step can be undone)
pub fn delete(id: &str) -> Result<(), String> {
    let p = path_of(id)?;
    read_only(&calendar_of(&p), id)?;
    let before = fs::read_to_string(&p).ok();
    fs::remove_file(&p).map_err(|x| x.to_string())?;
    undo::record(&calendar_of(&p), id, before, None);
    Ok(())
}

// ───── options ─────

#[derive(Default)]
struct Opts {
    words: Vec<String>,
    one: std::collections::HashMap<String, String>,
    alarms: Vec<String>,
    flags: Vec<String>,
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.strip_prefix("--") {
            Some(name @ ("reminder" | "tsv" | "json" | "undo" | "dry-run")) => o.flags.push(name.to_string()),
            Some("alarm") => o.alarms.push(it.next().ok_or("--alarm needs a duration")?.clone()),
            Some(name @ ("at" | "end" | "for" | "place" | "note" | "every" | "interval" | "count" | "until" | "title" | "on" | "from" | "to" | "days" | "calendar" | "refresh" | "file")) => {
                o.one.insert(name.to_string(), it.next().ok_or(format!("--{name} needs a value"))?.clone());
            }
            Some(other) => return Err(format!("unknown option: --{other}")),
            None => o.words.push(a.clone()),
        }
    }
    Ok(o)
}

/// Apply the options of add / edit to an event
fn apply(e: &mut Event, o: &Opts, now: NaiveDateTime, adding: bool) -> Result<(), String> {
    if let Some(t) = o.one.get("title") {
        e.summary.clone_from(t);
    }
    if let Some(c) = o.one.get("calendar") {
        e.calendar = calendars::find(c).ok_or_else(|| format!("no calendar named {c} (fili-cal calendar list)"))?.slug;
    }
    if let Some(p) = o.one.get("place") {
        e.place.clone_from(p);
    }
    if let Some(n) = o.one.get("note") {
        e.note.clone_from(n);
    }
    let old_len = e.length();
    if let Some(at) = o.one.get("at") {
        let (d, all) = when::parse_when(at, now)?;
        // typed times are the ones the views show; the event may live in another zone
        (e.start, e.all_day) = (if all { d } else { e.from_display(d) }, all);
        // the length of the event is kept when only the start moves
        if !adding && e.end.is_some() && !o.one.contains_key("end") && !o.one.contains_key("for") {
            e.end = Some(d + old_len);
        }
    }
    if let Some(end) = o.one.get("end") {
        // a time alone is on the day of the start (a duration is counted from the start)
        let base = if end.starts_with('+') { e.start } else { e.start.date().and_hms_opt(0, 0, 0).unwrap_or(e.start) };
        let (d, _) = when::parse_when(end, base)?;
        e.end = Some(if e.all_day || end.starts_with('+') { d } else { e.from_display(d) });
    }
    if let Some(len) = o.one.get("for") {
        e.end = Some(e.start + Duration::seconds(when::parse_duration(len)?));
    }
    if let Some(end) = e.end
        && end < e.start
    {
        return Err("the end is before the start".into());
    }
    if o.flags.iter().any(|f| f == "reminder") {
        e.alarms = vec![0];
    }
    if !o.alarms.is_empty() {
        e.alarms = o.alarms.iter().filter(|a| a.as_str() != "none").map(|a| when::parse_duration(a)).collect::<Result<_, _>>()?;
    } else if adding && !o.flags.iter().any(|f| f == "reminder") {
        e.alarms = if e.all_day { vec![] } else { crate::settings::default_alarms() };
    }
    if let Some(freq) = o.one.get("every") {
        e.rule = if freq == "none" {
            None
        } else {
            Some(Rule::simple(Freq::parse(freq).ok_or("--every: day, week, month, year or none")?, 1, None, None))
        };
    }
    if let Some(r) = e.rule.as_mut() {
        if let Some(i) = o.one.get("interval") {
            r.interval = i.parse::<u32>().ok().filter(|n| *n > 0).ok_or("--interval needs a positive number")?;
        }
        if let Some(c) = o.one.get("count") {
            r.count = Some(c.parse().map_err(|_| "--count needs a number")?);
        }
        if let Some(u) = o.one.get("until") {
            r.until = Some(when::parse_when(u, now)?.0);
        }
    } else if ["interval", "count", "until"].iter().any(|k| o.one.contains_key(*k)) {
        return Err("--interval, --count and --until need --every".into());
    }
    Ok(())
}

// ───── output ─────

fn when_text(d: NaiveDateTime, all_day: bool) -> String {
    if all_day { d.format("%Y-%m-%d").to_string() } else { d.format("%Y-%m-%d %H:%M").to_string() }
}

struct Occ<'a> {
    id: &'a str,
    ev: &'a Event,
    start: NaiveDateTime,
}

fn window<'a>(all: &'a [(String, Event)], from: NaiveDateTime, to: NaiveDateTime) -> Vec<Occ<'a>> {
    let mut v: Vec<Occ> = all.iter().flat_map(|(id, ev)| ics::occurrences(ev, from, to).into_iter().map(move |start| Occ { id, ev, start })).collect();
    v.sort_by(|a, b| (a.start, &a.ev.summary).cmp(&(b.start, &b.ev.summary)));
    v
}

fn print_occ(v: &[Occ], mode: &str) {
    if mode == "json" {
        let rows: Vec<_> = v
            .iter()
            .map(|o| {
                let end = o.ev.end_of(o.start);
                json!({"v": 1, "id": o.id, "title": o.ev.summary, "start": o.start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    "end": end.map(|e| e.format("%Y-%m-%dT%H:%M:%S").to_string()), "all_day": o.ev.all_day, "place": o.ev.place,
                    "note": o.ev.note, "alarms": o.ev.alarms, "every": o.ev.rule.as_ref().map(|r| r.freq.name()), "tz": o.ev.tzid, "calendar": if o.ev.calendar.is_empty() { calendars::DEFAULT } else { &o.ev.calendar }})
            })
            .collect();
        println!("{}", json!(rows));
        return;
    }
    for o in v {
        if mode == "tsv" {
            let end = o.ev.end_of(o.start).map(|e| when_text(e, o.ev.all_day)).unwrap_or_default();
            println!("{}\t{}\t{}\t{}\t{}\t{}", o.id, when_text(o.start, o.ev.all_day), end, o.ev.all_day, o.ev.summary, o.ev.place);
        } else {
            let place = if o.ev.place.is_empty() { String::new() } else { format!("  ({})", o.ev.place) };
            let every = o.ev.rule.as_ref().map_or(String::new(), |r| format!("  ↻ {}", r.freq.name().to_lowercase()));
            println!("{}  {}{}{}  [{}]", when_text(o.start, o.ev.all_day), o.ev.summary, place, every, o.id);
        }
    }
}

// ───── tick ─────

/// File of the clock card: `epoch<TAB>title<TAB>place` of the next timed events (whole days are not listed)
fn write_agenda(all: &[(String, Event)], now: NaiveDateTime) {
    let rows: String = window(all, now - Duration::hours(1), now + Duration::days(14))
        .iter()
        .filter(|o| !o.ev.all_day)
        .take(20)
        .map(|o| format!("{}\t{}\t{}\n", when::to_epoch(o.start), o.ev.summary.replace(['\t', '\n'], " "), o.ev.place.replace(['\t', '\n'], " ")))
        .collect();
    let dir = state();
    let _ = fs::create_dir_all(&dir);
    let tmp = dir.join("agenda.new");
    if fs::write(&tmp, rows).is_ok() {
        let _ = fs::rename(tmp, dir.join("agenda"));
    }
}

fn notify(title: &str, body: &str) {
    let cmd = env::var("FILI_CAL_NOTIFY").unwrap_or_else(|_| "notify-send".into());
    let mut c = Command::new(&cmd);
    if cmd == "notify-send" {
        c.args(["-a", "Agenda", "-i", "appointment-soon"]);
    }
    let _ = c.arg(title).arg(body).status();
}

/// An alert with buttons: Snooze 10 min, Snooze 1 hour, Open. The notification is held by a process of its own (`alert-wait`) until
/// it is answered or closed, so that the minute tick never waits. `FILI_CAL_NOTIFY` (tests) gets a plain notification.
fn alert(title: &str, body: &str, id: &str, occ: NaiveDateTime) {
    if env::var("FILI_CAL_NOTIFY").is_ok() {
        return notify(title, body);
    }
    if let Ok(exe) = env::current_exe() {
        let _ = Command::new(exe).args(["alert-wait", id, &when::to_epoch(occ).to_string(), title, body]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

/// `fili-cal alert-wait ID OCCURRENCE_EPOCH TITLE BODY`: shows the alert with its buttons and does what is chosen
fn alert_wait(w: &[String]) -> Result<(), String> {
    let [id, occ, title, body] = w else { return Err("alert-wait ID EPOCH TITLE BODY".into()) };
    let out = Command::new("notify-send")
        .args(["-a", "Agenda", "-i", "appointment-soon", "--wait", "-A"])
        .arg(format!("snooze10={}", i18n::tr("Snooze 10 min")))
        .arg("-A")
        .arg(format!("snooze60={}", i18n::tr("Snooze 1 hour")))
        .arg("-A")
        .arg(format!("open={}", i18n::tr("Open")))
        .arg(title)
        .arg(body)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let occ: i64 = occ.parse().map_err(|_| "bad epoch")?;
    match String::from_utf8_lossy(&out.stdout).trim() {
        "snooze10" => snooze(id, occ, 600),
        "snooze60" => snooze(id, occ, 3600),
        // the bubble itself was clicked, or the button
        "open" | "default" => open_on(when::from_epoch(occ)),
        _ => {}
    }
    Ok(())
}

fn snooze_path() -> PathBuf {
    state().join("fili-cal").join("snoozed")
}

/// Rings again in `secs` seconds (kept in the state folder, not in the event)
fn snooze(id: &str, occ: i64, secs: i64) {
    let due = when::to_epoch(when::now()) + secs;
    let _ = fs::create_dir_all(state().join("fili-cal"));
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(snooze_path()) {
        use std::io::Write;
        let _ = writeln!(f, "{due} {id} {occ}");
    }
}

/// Shows Agenda on the day of an event: the open window goes there, or a new one starts there
fn open_on(day: NaiveDateTime) {
    let date = day.format("%Y-%m-%d").to_string();
    if let Some(pid) = crate::ctl::windows().first()
        && crate::ctl::client(&["--pid".into(), pid.to_string(), "action".into(), "goto".into(), date.clone()]).is_ok()
    {
        crate::window::focus_pid(*pid);
        return;
    }
    if let Ok(exe) = env::current_exe() {
        let _ = Command::new(exe).env("FILI_CAL_GOTO", date).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

fn alert_body(ev: &Event, start: NaiveDateTime, before: i64) -> String {
    let place = if ev.place.is_empty() { String::new() } else { format!(" · {}", ev.place) };
    if ev.all_day {
        start.format("%Y-%m-%d").to_string()
    } else if before > 0 {
        format!("{:02}:{:02}, in {} min{place}", start.hour(), start.minute(), before / 60)
    } else {
        format!("{:02}:{:02}{place}", start.hour(), start.minute())
    }
}

/// Tells other tools (`fili-event watch calendar.alert`) that an alert went off; nobody has to listen
fn emit_alert(id: &str, title: &str, start: NaiveDateTime) {
    let data = json!({"id": id, "title": title, "start": start.format("%Y-%m-%dT%H:%M").to_string()}).to_string();
    let _ = Command::new("fili-event").args(["emit", "calendar.alert", &data]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

pub fn tick() -> Result<(), String> {
    // subscriptions that are due are fetched by a process of their own, so that the minute tick never waits for the network
    crate::subscribe::spawn_due();
    let now = when::now();
    let all = load_all();
    let fired_path = state().join("calendar-fired");
    let fired_text = fs::read_to_string(&fired_path).unwrap_or_default();
    let mut fired: Vec<String> = fired_text.lines().map(str::to_string).collect();
    let (horizon, catch_up) = (Duration::days(1), Duration::hours(12));
    let mut changed = false;
    // what rings in this tick: (id, title, body, start of the occurrence, minutes late)
    let mut ringing: Vec<(String, String, String, NaiveDateTime, i64)> = Vec::new();
    // snoozed alerts that are due
    let snoozed = fs::read_to_string(snooze_path()).unwrap_or_default();
    let now_epoch = when::to_epoch(now);
    let mut still: Vec<&str> = Vec::new();
    for line in snoozed.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (Some(due), Some(id), Some(occ)) = (f.first().and_then(|d| d.parse::<i64>().ok()), f.get(1), f.get(2).and_then(|o| o.parse::<i64>().ok())) else { continue };
        if due > now_epoch {
            still.push(line);
        } else if let Some((_, ev)) = all.iter().find(|(i, _)| i == id) {
            let start = when::from_epoch(occ);
            ringing.push(((*id).to_string(), ev.summary.clone(), alert_body(ev, start, 0), start, 0));
        }
    }
    if still.len() != snoozed.lines().count() {
        let _ = fs::write(snooze_path(), still.iter().map(|l| format!("{l}\n")).collect::<String>());
    }
    for (id, ev) in &all {
        let longest = ev.alarms.iter().copied().max().unwrap_or(0).max(0);
        // an alarm that is due started between (now - catch_up) and now, for an occurrence that starts at most `longest` later
        let from = now - catch_up - Duration::hours(24);
        for start in ics::occurrences(ev, from, now + Duration::seconds(longest) + horizon) {
            // the alerts of the event, and the one for the trip when the event has a place (an option: `travel_time`)
            let travel = if ev.place.is_empty() || ev.all_day { 0 } else { crate::settings::travel_time() };
            let mut alarms = ev.alarms.clone();
            if travel > 0 && !alarms.contains(&travel) {
                alarms.push(travel);
            }
            for a in &alarms {
                // the alarm of a whole day goes off at 09:00 that day
                let base = if ev.all_day { start.date().and_hms_opt(crate::settings::all_day_hour(), 0, 0).unwrap_or(start) } else { start };
                let at = base - Duration::seconds(*a);
                if at > now || at < now - catch_up {
                    continue;
                }
                let key = format!("{id} {}", when::to_epoch(at));
                if fired.contains(&key) {
                    continue;
                }
                let body = if travel > 0 && *a == travel { i18n::trf("Time to leave for %1", &[&alert_body(ev, start, *a)]) } else { alert_body(ev, start, *a) };
                ringing.push((id.clone(), ev.summary.clone(), body, start, (now - at).num_minutes()));
                fired.push(key);
                changed = true;
            }
        }
    }
    // After a suspend or a shutdown many alerts can be late at once: one notice for all of them, never a storm
    let late: Vec<&(String, String, String, NaiveDateTime, i64)> = ringing.iter().filter(|r| r.4 >= 5).collect();
    if late.len() > 2 {
        let titles: Vec<&str> = late.iter().take(4).map(|r| r.1.as_str()).collect();
        notify(&i18n::trf("%1 alerts you missed", &[&late.len()]), &(titles.join(" · ") + if late.len() > 4 { " …" } else { "" }));
        ringing.retain(|r| r.4 < 5);
    }
    for (id, title, body, start, _) in &ringing {
        alert(title, body, id, *start);
        emit_alert(id, title, *start);
    }
    // forget the old ones (their alarm is older than 3 days)
    let cutoff = when::to_epoch(now - Duration::days(3));
    let kept: Vec<String> = fired.into_iter().filter(|k| k.rsplit(' ').next().and_then(|e| e.parse::<i64>().ok()).is_some_and(|e| e >= cutoff)).collect();
    if changed || kept.len() != fired_text.lines().count() {
        let _ = fs::create_dir_all(state());
        let _ = fs::write(&fired_path, kept.join("\n") + "\n");
    }
    write_agenda(&all, now);
    Ok(())
}

// ───── commands ─────

/// One command is one step of the journal: `fili-cal undo` takes it back
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("alert-wait") => alert_wait(&args[1..])?,
        Some("undo") => println!("{}", undo::undo()?),
        Some("redo") => println!("{}", undo::redo()?),
        _ => {
            let done = run_command(args);
            let what = args.iter().skip(1).find(|a| !a.starts_with('-')).map_or(String::new(), |w| format!(" {w}"));
            undo::commit(&format!("{}{what}", args.first().map_or("", String::as_str)));
            return done;
        }
    }
    Ok(())
}

fn run_command(args: &[String]) -> Result<(), String> {
    let now = when::now();
    let cmd = args.first().map_or("list", String::as_str);
    let o = parse_opts(args.get(1..).unwrap_or(&[]))?;
    match cmd {
        "add" => {
            let title = o.words.join(" ");
            if title.trim().is_empty() {
                return Err("add needs a title".into());
            }
            if !o.one.contains_key("at") {
                return Err("add needs --at WHEN".into());
            }
            let id = new_id();
            let mut e = Event { uid: format!("{id}@filigrane"), summary: title, start: now, ..Event::default() };
            apply(&mut e, &o, now, true)?;
            save(&id, &e)?;
            println!("{id}");
            let _ = tick();
        }
        "quick" => {
            let words = &o.words;
            // the longest start of the line that is a date (a "à" or "at" between the day and the hour is ignored)
            let found = (1..=words.len().min(3)).rev().find_map(|n| {
                let head: Vec<&str> = words[..n].iter().map(String::as_str).enumerate().filter(|(i, w)| !(*i == 1 && matches!(*w, "à" | "at"))).map(|(_, w)| w).collect();
                when::parse_when(&head.join(" "), now).ok().map(|w| (n, w))
            });
            let Some((n, (start, all_day))) = found else {
                return Err("quick: the line starts with a date (14:30, tomorrow 09:00, 2026-12-24, +30m...), then the title".into());
            };
            let title = words[n..].join(" ");
            if title.trim().is_empty() {
                return Err("quick: the title is missing".into());
            }
            let id = new_id();
            let alarms = if words[0].starts_with('+') { vec![0] } else if all_day { vec![] } else { crate::settings::default_alarms() };
            let calendar = match o.one.get("calendar") {
                Some(c) => calendars::find(c).ok_or_else(|| format!("no calendar named {c}"))?.slug,
                None => String::new(),
            };
            let e = Event { uid: format!("{id}@filigrane"), summary: title, start, all_day, alarms, calendar, ..Event::default() };
            save(&id, &e)?;
            println!("{id}");
            let _ = tick();
        }
        "edit" => {
            let id = o.words.first().ok_or("edit needs an id")?;
            let mut e = load(id)?;
            apply(&mut e, &o, now, false)?;
            save(id, &e)?;
            let _ = tick();
        }
        "rm" => {
            let id = o.words.first().ok_or("rm needs an id")?;
            let mut e = load(id)?;
            if let Some(on) = o.one.get("on") {
                let (d, _) = when::parse_when(on, e.display_start())?;
                let d = if e.all_day { d.date().and_hms_opt(0, 0, 0).unwrap_or(d) } else { d };
                if e.rule.is_none() || !ics::occurrences(&e, d, d + Duration::seconds(1)).contains(&d) {
                    return Err(format!("no occurrence of {id} on {on}"));
                }
                e.skip.push(e.from_display(d));
                save(id, &e)?;
            } else {
                delete(id)?;
            }
            let _ = tick();
        }
        "show" => {
            let id = o.words.first().ok_or("show needs an id")?;
            let e = load(id)?;
            print!("{}", ics::to_ics(&e).replace("\r\n", "\n"));
        }
        "list" | "next" => {
            let mut all = load_all();
            if let Some(c) = o.one.get("calendar") {
                let slug = calendars::find(c).ok_or_else(|| format!("no calendar named {c}"))?.slug;
                all.retain(|(_, e)| (if e.calendar.is_empty() { calendars::DEFAULT } else { &e.calendar }) == slug);
            }
            let mode = if o.flags.iter().any(|f| f == "json") { "json" } else if o.flags.iter().any(|f| f == "tsv") { "tsv" } else { "text" };
            let today = now.date().and_hms_opt(0, 0, 0).unwrap_or(now);
            if cmd == "next" {
                let n: usize = o.words.first().map_or(Ok(5), |s| s.parse()).map_err(|_| "next takes a number")?;
                let v = window(&all, now, now + Duration::days(366 * 2));
                print_occ(&v[..v.len().min(n)], mode);
            } else {
                let from = o.one.get("from").map(|f| when::parse_when(f, now)).transpose()?.map_or(today, |p| p.0);
                let to = match (o.one.get("to"), o.one.get("days")) {
                    (Some(t), _) => when::parse_when(t, now)?.0,
                    (None, Some(d)) => from + Duration::days(d.parse().map_err(|_| "--days needs a number")?),
                    _ => from + Duration::days(31),
                };
                print_occ(&window(&all, from, to), mode);
            }
        }
        "tick" => tick()?,
        "free" => free(&o, now)?,
        "reply" => {
            let (src, ans) = (o.words.first().ok_or("reply needs a file or a link")?, o.words.get(1).ok_or("reply needs accept, maybe or decline")?);
            let answer = invites::Answer::parse(ans).ok_or("the answer is accept, maybe or decline")?;
            println!("{}", invites::reply(src, answer, o.one.get("calendar").map_or("", String::as_str), o.flags.iter().any(|f| f == "dry-run"))?);
        }
        "done" => {
            let id = o.words.first().ok_or("done needs an id")?;
            let mut e = load(id)?;
            e.set_done(!o.flags.iter().any(|f| f == "undo"));
            save(id, &e)?;
        }
        "calendar" => calendar_command(&o.words, o.one.get("refresh").and_then(|r| r.parse().ok()))?,
        "import" => {
            let file = o.words.first().ok_or("import needs a file or a link")?;
            let (added, replaced) = exchange::import(file, o.one.get("calendar").map_or("", String::as_str))?;
            println!("{}", added + replaced);
        }
        "export" => {
            let text = match o.words.first() {
                Some(id) => {
                    // one event
                    ics::to_ics(&load(id)?)
                }
                None => exchange::export(o.one.get("calendar").map(String::as_str))?,
            };
            match o.one.get("file") {
                Some(f) => println!("{}", exchange::export_to_text(&text, f)?),
                None => print!("{text}"),
            }
        }
        "-h" | "--help" | "help" => {
            println!("fili-cal add|edit|rm|show|list|next|tick|import|export|undo|redo (see the header of apps/fili-cal/src/main.rs)");
        }
        other => return Err(format!("unknown command: {other} (add, quick, edit, rm, show, list, next, tick, import, export)")),
    }
    Ok(())
}

/// `fili-cal free [--on WHEN] [--for 1h] [--from 09:00] [--to 18:00] [--json]`: the free slots of a day (read-only: what the
/// assistant uses to find "a free hour on Thursday afternoon"; adding the event stays an explicit request)
fn free(o: &Opts, now: NaiveDateTime) -> Result<(), String> {
    let day = o.one.get("on").map(|w| when::parse_when(w, now)).transpose()?.map_or(now, |p| p.0).date();
    let length = Duration::seconds(o.one.get("for").map_or(Ok(3600), |l| when::parse_duration(l))?);
    let clock = |key: &str, default: (u32, u32)| -> Result<NaiveDateTime, String> {
        let (h, m) = match o.one.get(key) {
            Some(t) => {
                let (h, m) = t.split_once(':').ok_or(format!("--{key} needs HH:MM"))?;
                (h.parse().map_err(|_| format!("--{key} needs HH:MM"))?, m.parse().map_err(|_| format!("--{key} needs HH:MM"))?)
            }
            None => default,
        };
        day.and_hms_opt(h, m, 0).ok_or(format!("--{key} needs HH:MM"))
    };
    let (open, close) = (clock("from", (9, 0))?, clock("to", (18, 0))?);
    let (from, to) = (day.and_hms_opt(0, 0, 0).unwrap_or(now), day.and_hms_opt(0, 0, 0).unwrap_or(now) + Duration::days(1));
    let mut busy: Vec<(NaiveDateTime, NaiveDateTime)> = load_all()
        .iter()
        .filter(|(_, e)| !e.all_day)
        .flat_map(|(_, e)| ics::occurrences(e, from, to).into_iter().map(move |s| (s, s + e.length().max(Duration::minutes(1)))))
        .collect();
    busy.sort();
    let mut slots = Vec::new();
    let mut cursor = open;
    for (s, e) in busy {
        if s > cursor && s.min(close) - cursor >= length {
            slots.push((cursor, s.min(close)));
        }
        cursor = cursor.max(e);
    }
    if close > cursor && close - cursor >= length {
        slots.push((cursor, close));
    }
    if o.flags.iter().any(|f| f == "json") {
        let rows: Vec<_> = slots.iter().map(|(a, b)| json!({"start": a.format("%Y-%m-%dT%H:%M").to_string(), "end": b.format("%Y-%m-%dT%H:%M").to_string()})).collect();
        println!("{}", json!(rows));
    } else if slots.is_empty() {
        println!("{}", i18n::tr("no free slot"));
    } else {
        for (a, b) in slots {
            println!("{} {}-{}", day.format("%Y-%m-%d"), a.format("%H:%M"), b.format("%H:%M"));
        }
    }
    Ok(())
}

/// The password of `calendar connect`: the first line of the standard input (nothing is shown while it is typed on a terminal)
fn read_password() -> Result<String, String> {
    use std::io::{IsTerminal, Write};
    let tty = std::io::stdin().is_terminal();
    if tty {
        eprint!("{}", crate::i18n::tr("Password: "));
        let _ = std::io::stderr().flush();
        let _ = std::process::Command::new("stty").arg("-echo").status();
    }
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    if tty {
        let _ = std::process::Command::new("stty").arg("echo").status();
        eprintln!();
    }
    read.map_err(|e| e.to_string())?;
    let pw = line.trim_end_matches(['\n', '\r']).to_string();
    if pw.is_empty() { Err(crate::i18n::tr("the password is empty").to_string()) } else { Ok(pw) }
}

/// `fili-cal calendar list|connect ADDRESS USER (password on the standard input)|add NAME [COLOR]|subscribe LINK NAME [COLOR] [--refresh MINUTES]|sync [NAME]|rename NAME NEW|color NAME TONE|hide NAME|show NAME|rm NAME`
fn calendar_command(w: &[String], refresh: Option<u32>) -> Result<(), String> {
    let need = |i: usize, what: &str| w.get(i).cloned().ok_or(format!("calendar: {what} is missing"));
    let slug = |i: usize| -> Result<String, String> {
        let name = need(i, "the calendar")?;
        Ok(calendars::find(&name).ok_or_else(|| format!("no calendar named {name}"))?.slug)
    };
    match w.first().map(String::as_str).unwrap_or("list") {
        "list" => {
            for c in calendars::list() {
                println!("{}\t{}\t{}{}{}", c.slug, c.name, c.color, if c.url.is_some() { "\tsubscription" } else if c.caldav.is_some() { "\taccount" } else { "" }, if c.hidden { "\thidden" } else { "" });
            }
        }
        "add" => println!("{}", calendars::create(&need(1, "the name")?, w.get(2).map_or("", String::as_str))?.slug),
        "subscribe" => {
            let c = calendars::subscribe(&need(2, "the name")?, &need(1, "the link")?, w.get(3).map_or("", String::as_str), refresh)?;
            println!("{}", c.slug);
            // the first fetch happens now, in the background
            subscribe::spawn_due();
        }
        "sync" => subscribe::sync_command(w.get(1).map(String::as_str))?,
        "connect" => {
            let (address, user) = (need(1, "the address")?, need(2, "the user name")?);
            let pw = read_password()?;
            let made = dav::connect(&dav::Curl::new(&user, &pw), &address, &user, &pw, true)?;
            for n in &made {
                println!("{n}");
            }
            // the first fetch happens now, in the background
            subscribe::spawn_due();
        }
        "rename" => calendars::update(&slug(1)?, "name", &need(2, "the new name")?)?,
        "color" => calendars::update(&slug(1)?, "color", &need(2, "the tone")?)?,
        "hide" => calendars::update(&slug(1)?, "hidden", "true")?,
        "show" => calendars::update(&slug(1)?, "hidden", "false")?,
        "rm" => {
            let s = slug(1)?;
            calendars::remove(&s)?;
            dav::forget_password(&s);
        }
        other => return Err(format!("calendar: unknown command {other} (list, add, rename, color, hide, show, rm)")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snoozed_alert_rings_again_when_due_and_is_then_forgotten() {
        let _turn = STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-snooze-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the tests that set the store take turns (STORE_LOCK)
        unsafe {
            env::set_var("XDG_DATA_HOME", tmp.join("data"));
            env::set_var("XDG_STATE_HOME", tmp.join("state"));
            env::set_var("FILI_CAL_NOW", "2026-12-01T09:30");
            // notifications go to a program that does nothing
            env::set_var("FILI_CAL_NOTIFY", "true");
        }
        let e = Event { uid: "s@x".into(), summary: "Call".into(), start: when::now().date().and_hms_opt(8, 0, 0).unwrap(), ..Event::default() };
        save("s1", &e).unwrap();
        // the change is recorded in a process-wide list: close it here so that no other test takes it
        undo::commit("test");
        let occ = when::to_epoch(e.start);
        let now = when::to_epoch(when::now());
        // one snoozed alert due a minute ago, one due in an hour
        fs::create_dir_all(snooze_path().parent().unwrap()).unwrap();
        fs::write(snooze_path(), format!("{} s1 {occ}\n{} s1 {occ}\n", now - 60, now + 3600)).unwrap();
        tick().unwrap();
        let left = fs::read_to_string(snooze_path()).unwrap();
        assert_eq!(left.lines().count(), 1);
        assert!(left.starts_with(&format!("{}", now + 3600)));
        assert_eq!(alert_body(&e, e.start, 600), "08:00, in 10 min");
        let _ = fs::remove_dir_all(&tmp);
    }
}
