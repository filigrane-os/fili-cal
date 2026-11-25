//! The events: model, iCalendar (RFC 5545) reading and writing, and recurrence. An event keeps the wall time of its own
//! zone (`tzid`; none = floating, the hour on the wall of whoever reads it), so a recurring event stays at the same hour
//! across daylight saving time; the views get the times converted to the system zone (see `zone`). A reminder is an event
//! with an alarm at its start. What this program does not understand in a file (rule parts, properties) is kept and written back.
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::zone;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// A recurrence rule (RFC 5545 section 3.3.10). `by_day` holds (ordinal, weekday), ordinal 0 = every such weekday.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<NaiveDateTime>,
    pub by_day: Vec<(i32, Weekday)>,
    pub by_monthday: Vec<i32>,
    pub by_month: Vec<u32>,
    pub by_setpos: Vec<i32>,
    pub wkst: Weekday,
    /// Rule parts that are not applied (BYYEARDAY, BYWEEKNO, BYHOUR…), written back as they came ("KEY=VALUE")
    pub extra: Vec<String>,
}

impl Rule {
    pub fn simple(freq: Freq, interval: u32, count: Option<u32>, until: Option<NaiveDateTime>) -> Rule {
        Rule { freq, interval, count, until, by_day: vec![], by_monthday: vec![], by_month: vec![], by_setpos: vec![], wkst: Weekday::Mon, extra: vec![] }
    }
    /// Has parts the editor cannot show as "every N days / weeks / months / years" (the rule is then "custom")
    pub fn is_custom(&self, start: NaiveDateTime) -> bool {
        let day_ok = self.by_day.is_empty() || (self.freq == Freq::Weekly && self.by_day == [(0, start.weekday())]);
        let monthday_ok = self.by_monthday.is_empty() || (self.freq == Freq::Monthly && self.by_monthday == [i32::try_from(start.day()).unwrap_or(0)]);
        let month_ok = self.by_month.is_empty() || (self.freq == Freq::Yearly && self.by_month == [start.month()]);
        !(day_ok && monthday_ok && month_ok) || !self.by_setpos.is_empty() || !self.extra.is_empty()
    }
    /// The text of the RRULE property
    pub fn text(&self, all_day: bool) -> String {
        let mut s = format!("FREQ={};INTERVAL={}", self.freq.name(), self.interval);
        if let Some(c) = self.count {
            s += &format!(";COUNT={c}");
        }
        if let Some(u) = self.until {
            s += &if all_day { format!(";UNTIL={}", u.format("%Y%m%d")) } else { format!(";UNTIL={}", fmt_dt(u)) };
        }
        if !self.by_day.is_empty() {
            let days: Vec<String> = self.by_day.iter().map(|(n, w)| if *n == 0 { weekday_code(*w).to_string() } else { format!("{n}{}", weekday_code(*w)) }).collect();
            s += &format!(";BYDAY={}", days.join(","));
        }
        let list = |name: &str, v: &[i64]| if v.is_empty() { String::new() } else { format!(";{name}={}", v.iter().map(i64::to_string).collect::<Vec<_>>().join(",")) };
        s += &list("BYMONTHDAY", &self.by_monthday.iter().map(|n| i64::from(*n)).collect::<Vec<_>>());
        s += &list("BYMONTH", &self.by_month.iter().map(|n| i64::from(*n)).collect::<Vec<_>>());
        s += &list("BYSETPOS", &self.by_setpos.iter().map(|n| i64::from(*n)).collect::<Vec<_>>());
        if self.wkst != Weekday::Mon {
            s += &format!(";WKST={}", weekday_code(self.wkst));
        }
        for x in &self.extra {
            s += &format!(";{x}");
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub uid: String,
    pub summary: String,
    pub place: String,
    pub note: String,
    /// Wall time in the zone of the event (`tzid`). For a whole-day event, midnight of its first day.
    pub start: NaiveDateTime,
    /// Wall time of the end in the same zone; for a whole-day event the day after the last one (DTEND is exclusive)
    pub end: Option<NaiveDateTime>,
    pub all_day: bool,
    /// Zone of the times: a name of the zone database, "UTC", or None for a floating event
    pub tzid: Option<String>,
    /// Seconds before the start (0 = at the start, negative = after it)
    pub alarms: Vec<i64>,
    pub rule: Option<Rule>,
    /// Occurrences that were deleted one by one (the start of the occurrence, in the zone of the event)
    pub skip: Vec<NaiveDateTime>,
    /// Occurrences added by hand (RDATE)
    pub rdate: Vec<NaiveDateTime>,
    /// Properties of the VEVENT this program does not use (ATTENDEE, CATEGORIES, X-…), written back unchanged
    pub extra: Vec<String>,
    /// The calendar (folder of the store) that holds the file; not part of the iCalendar text, empty = the default one
    pub calendar: String,
}

impl Default for Event {
    fn default() -> Event {
        Event {
            uid: String::new(),
            summary: String::new(),
            place: String::new(),
            note: String::new(),
            start: chrono::DateTime::UNIX_EPOCH.naive_utc(),
            end: None,
            all_day: false,
            tzid: None,
            alarms: vec![],
            rule: None,
            skip: vec![],
            rdate: vec![],
            extra: vec![],
            calendar: String::new(),
        }
    }
}

fn weekday_code(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}
fn parse_weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => return None,
    })
}

impl Freq {
    pub fn name(self) -> &'static str {
        match self {
            Freq::Daily => "DAILY",
            Freq::Weekly => "WEEKLY",
            Freq::Monthly => "MONTHLY",
            Freq::Yearly => "YEARLY",
        }
    }
    pub fn parse(s: &str) -> Option<Freq> {
        Some(match s.to_lowercase().as_str() {
            "daily" | "day" | "d" => Freq::Daily,
            "weekly" | "week" | "w" => Freq::Weekly,
            "monthly" | "month" | "m" => Freq::Monthly,
            "yearly" | "year" | "y" => Freq::Yearly,
            _ => return None,
        })
    }
}

impl Event {
    /// Length of one occurrence
    pub fn length(&self) -> Duration {
        self.end.map_or(Duration::zero(), |e| e - self.start)
    }
    /// The end of the occurrence that the views show as starting at `shown`
    pub fn end_of(&self, shown: NaiveDateTime) -> Option<NaiveDateTime> {
        self.end.map(|_| shown + self.length())
    }
    /// The first start as the views show it (system zone)
    pub fn display_start(&self) -> NaiveDateTime {
        self.display(self.start)
    }
    /// A wall time of the zone of the event as the views show it. A whole-day event has no zone.
    pub fn display(&self, wall: NaiveDateTime) -> NaiveDateTime {
        if self.all_day { wall } else { zone::to_display(wall, self.tzid.as_deref()) }
    }
    /// A time shown by the views as a wall time of the zone of the event
    pub fn from_display(&self, shown: NaiveDateTime) -> NaiveDateTime {
        if self.all_day { shown } else { zone::from_display(shown, self.tzid.as_deref()) }
    }
}

// ───── text ─────

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n")
}
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

/// Lines folded at 75 octets, continuation lines start with a space
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut len = 0;
    for c in line.chars() {
        let n = c.len_utf8();
        if len + n > 74 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(c);
        len += n;
    }
    out
}

fn fmt_dt(d: NaiveDateTime) -> String {
    d.format("%Y%m%dT%H%M%S").to_string()
}
fn fmt_trigger(secs: i64) -> String {
    let (sign, n) = if secs > 0 { ("-", secs) } else { ("", -secs) };
    if n == 0 {
        return "PT0S".into();
    }
    let (d, h, m, s) = (n / 86400, n % 86400 / 3600, n % 3600 / 60, n % 60);
    let mut t = String::new();
    if h > 0 {
        t += &format!("{h}H");
    }
    if m > 0 {
        t += &format!("{m}M");
    }
    if s > 0 {
        t += &format!("{s}S");
    }
    format!("{sign}P{}{}", if d > 0 { format!("{d}D") } else { String::new() }, if t.is_empty() { String::new() } else { format!("T{t}") })
}

pub fn to_ics(e: &Event) -> String {
    let mut l: Vec<String> = vec!["BEGIN:VCALENDAR".into(), "VERSION:2.0".into(), "PRODID:-//Filigrane//fili-cal//EN".into(), "BEGIN:VEVENT".into()];
    l.push(format!("UID:{}", e.uid));
    l.push(format!("DTSTAMP:{}Z", chrono::Utc::now().format("%Y%m%dT%H%M%S")));
    // DTSTART, DTEND, EXDATE and RDATE share one notation: a date, a UTC time, a time with its zone, or a floating time
    let stamp = |name: &str, d: NaiveDateTime| -> String {
        if e.all_day {
            format!("{name};VALUE=DATE:{}", d.format("%Y%m%d"))
        } else {
            match e.tzid.as_deref() {
                None => format!("{name}:{}", fmt_dt(d)),
                Some(z) if z.eq_ignore_ascii_case("utc") => format!("{name}:{}Z", fmt_dt(d)),
                Some(z) => format!("{name};TZID={z}:{}", fmt_dt(d)),
            }
        }
    };
    l.push(stamp("DTSTART", e.start));
    if let Some(end) = e.end {
        l.push(stamp("DTEND", end));
    }
    l.push(format!("SUMMARY:{}", escape(&e.summary)));
    if !e.place.is_empty() {
        l.push(format!("LOCATION:{}", escape(&e.place)));
    }
    if !e.note.is_empty() {
        l.push(format!("DESCRIPTION:{}", escape(&e.note)));
    }
    if let Some(r) = &e.rule {
        l.push(format!("RRULE:{}", r.text(e.all_day)));
    }
    for x in &e.skip {
        l.push(stamp("EXDATE", *x));
    }
    for x in &e.rdate {
        l.push(stamp("RDATE", *x));
    }
    l.extend(e.extra.iter().cloned());
    for a in &e.alarms {
        l.push("BEGIN:VALARM".into());
        l.push("ACTION:DISPLAY".into());
        l.push(format!("DESCRIPTION:{}", escape(&e.summary)));
        l.push(format!("TRIGGER:{}", fmt_trigger(*a)));
        l.push("END:VALARM".into());
    }
    l.push("END:VEVENT".into());
    l.push("END:VCALENDAR".into());
    l.iter().map(|x| fold(x) + "\r\n").collect()
}

// ───── reading ─────

/// A date or a date-time of a property: (time, whole day, zone). A `Z` time is in "UTC", a `TZID` parameter names the zone.
fn parse_dt(name: &str, value: &str) -> Option<(NaiveDateTime, bool, Option<String>)> {
    let v = value.trim();
    if v.len() == 8 {
        return Some((NaiveDate::parse_from_str(v, "%Y%m%d").ok()?.and_time(NaiveTime::MIN), true, None));
    }
    let dt = NaiveDateTime::parse_from_str(v.trim_end_matches('Z'), "%Y%m%dT%H%M%S").ok()?;
    if v.ends_with('Z') {
        return Some((dt, false, Some("UTC".into())));
    }
    let tzid = name.split(';').skip(1).find_map(|p| p.split_once('=').filter(|(k, _)| k.eq_ignore_ascii_case("TZID")).map(|(_, z)| z.trim_matches('"').to_string()));
    Some((dt, false, tzid))
}

/// "-PT10M", "PT0S", "-P1D", "-P1W", "-PT1H30M" → seconds before the start
fn parse_trigger(v: &str) -> Option<i64> {
    let v = v.trim();
    let (neg, v) = match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v.strip_prefix('+').unwrap_or(v)),
    };
    let v = v.strip_prefix('P')?;
    let (mut total, mut num) = (0i64, String::new());
    for c in v.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        if c == 'T' {
            continue;
        }
        let n: i64 = num.parse().ok()?;
        num.clear();
        total += n * match c {
            'W' => 604800,
            'D' => 86400,
            'H' => 3600,
            'M' => 60,
            'S' => 1,
            _ => return None,
        };
    }
    Some(if neg { total } else { -total })
}

/// The RRULE of an event. A malformed value yields None (the rule is dropped, the event stays a single one).
fn parse_rule(v: &str, zone_of: impl Fn(NaiveDateTime, Option<String>, bool) -> NaiveDateTime) -> Option<Rule> {
    let mut r = Rule::simple(Freq::Daily, 1, None, None);
    let mut freq = None;
    for part in v.split(';').filter(|p| !p.trim().is_empty()) {
        let (k, val) = part.split_once('=')?;
        let ints = |v: &str| v.split(',').map(|n| n.trim().trim_start_matches('+').parse::<i32>().ok()).collect::<Option<Vec<i32>>>();
        match k.to_uppercase().as_str() {
            "FREQ" => freq = Some(Freq::parse(val)?),
            "INTERVAL" => r.interval = val.parse::<u32>().ok()?.max(1),
            "COUNT" => r.count = Some(val.parse().ok()?),
            "UNTIL" => {
                let (d, all, tz) = parse_dt("", val)?;
                r.until = Some(zone_of(d, tz, all));
            }
            "BYDAY" => {
                for tok in val.split(',') {
                    let tok = tok.trim();
                    let (n, code) = tok.split_at(tok.len().checked_sub(2)?);
                    let n = if n.is_empty() { 0 } else { n.trim_start_matches('+').parse().ok()? };
                    r.by_day.push((n, parse_weekday(code)?));
                }
            }
            "BYMONTHDAY" => r.by_monthday = ints(val)?,
            "BYMONTH" => r.by_month = ints(val)?.into_iter().filter_map(|n| u32::try_from(n).ok()).collect(),
            "BYSETPOS" => r.by_setpos = ints(val)?,
            "WKST" => r.wkst = parse_weekday(val)?,
            _ => r.extra.push(format!("{k}={val}")),
        }
    }
    r.freq = freq?;
    Some(r)
}

fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        if (raw.starts_with(' ') || raw.starts_with('\t')) && !lines.is_empty() {
            lines.last_mut().unwrap().push_str(&raw[1..]);
        } else {
            lines.push(raw.to_string());
        }
    }
    lines
}

/// An alarm being read: its trigger and its repetition
#[derive(Default)]
struct Alarm {
    /// Seconds before the start (relative to the start), or the absolute time of the trigger
    relative: Option<i64>,
    related_end: bool,
    absolute: Option<(NaiveDateTime, Option<String>)>,
    repeat: i64,
    every: i64,
}

/// What is read of a VEVENT before it can be finished: values that depend on others (the duration on the start, an absolute alarm on the zone)
#[derive(Default)]
struct Pending {
    duration: Option<i64>,
    alarms: Vec<Alarm>,
    recurrence_id: Option<(NaiveDateTime, Option<String>)>,
    rrule: Option<String>,
    exdates: Vec<(NaiveDateTime, Option<String>)>,
    rdates: Vec<(NaiveDateTime, Option<String>)>,
    end: Option<(NaiveDateTime, Option<String>)>,
}

const KNOWN: [&str; 12] = ["UID", "SUMMARY", "LOCATION", "DESCRIPTION", "DTSTART", "DTEND", "DURATION", "RRULE", "EXDATE", "RDATE", "DTSTAMP", "RECURRENCE-ID"];

fn finish(mut e: Event, p: Pending, out: &mut Vec<Event>, overrides: &mut Vec<(String, NaiveDateTime, Event)>) {
    if e.uid.is_empty() {
        e.uid = format!("{}@import", crate::short_hash(&format!("{}{}", e.summary, e.start)));
    }
    // times of other zones than the one of the start are brought to it
    let into = |d: NaiveDateTime, tz: Option<String>| if e.all_day { d } else { zone::convert(d, tz.as_deref().or(e.tzid.as_deref()), e.tzid.as_deref()) };
    e.end = match (p.end, p.duration) {
        (Some((d, tz)), _) => Some(into(d, tz)),
        (None, Some(secs)) => Some(e.start + Duration::seconds(secs)),
        // a whole-day event without an end lasts one day (RFC 5545), which a start alone already says
        _ => None,
    };
    if let Some(text) = &p.rrule {
        let (tzid, all) = (e.tzid.clone(), e.all_day);
        e.rule = parse_rule(text, |d, tz, d_all| if all || d_all { d } else { zone::convert(d, tz.as_deref().or(tzid.as_deref()), tzid.as_deref()) });
        // a whole-day UNTIL, on a timed event, lasts until the end of that day
        if let (Some(r), false) = (e.rule.as_mut(), e.all_day)
            && text.to_uppercase().split(';').any(|x| x.starts_with("UNTIL=") && x.len() == 14)
            && let Some(u) = r.until
        {
            r.until = Some(u + Duration::hours(23) + Duration::minutes(59) + Duration::seconds(59));
        }
    }
    e.skip = p.exdates.into_iter().map(|(d, tz)| into(d, tz)).collect();
    e.rdate = p.rdates.into_iter().map(|(d, tz)| into(d, tz)).collect();
    let len = e.length().num_seconds();
    let start_epoch = zone::wall_to_epoch(e.start, e.tzid.as_deref());
    for a in p.alarms {
        let first = match (a.relative, a.absolute) {
            (Some(s), _) => Some(if a.related_end { s - len } else { s }),
            (None, Some((d, tz))) => Some(start_epoch - zone::wall_to_epoch(d, tz.as_deref().or(Some("UTC")))),
            _ => None,
        };
        if let Some(f) = first {
            e.alarms.push(f);
            // a repeated alarm rings again every `every` seconds, `repeat` times: later means fewer seconds before the start
            for i in 1..=a.repeat.clamp(0, 20) {
                e.alarms.push(f - i * a.every);
            }
        }
    }
    e.alarms.dedup();
    match p.recurrence_id {
        // a changed occurrence of a series: it becomes an event of its own and the series skips that occurrence
        Some((d, tz)) => {
            let master_tz = tz.or_else(|| e.tzid.clone());
            overrides.push((e.uid.clone(), d, Event { uid: format!("{}-{}", e.uid, d.format("%Y%m%dT%H%M%S")), tzid: e.tzid.clone().or(master_tz), ..e }));
        }
        None => out.push(e),
    }
}

/// Every VEVENT of an iCalendar text (a VEVENT that has no start is left out)
pub fn parse_ics(text: &str) -> Vec<Event> {
    let mut out = Vec::new();
    let mut overrides: Vec<(String, NaiveDateTime, Event)> = Vec::new();
    let mut cur: Option<(Event, Pending)> = None;
    let mut alarm: Option<Alarm> = None;
    // nested components other than VALARM (a VTIMEZONE is not inside a VEVENT) are skipped until their END
    let mut depth_skip = 0u32;
    for line in unfold(text) {
        let Some((name, value)) = line.split_once(':') else { continue };
        let prop = name.split(';').next().unwrap_or("").to_uppercase();
        if depth_skip > 0 {
            match prop.as_str() {
                "BEGIN" => depth_skip += 1,
                "END" => depth_skip -= 1,
                _ => {}
            }
            continue;
        }
        match (prop.as_str(), cur.as_mut()) {
            ("BEGIN", _) if value == "VEVENT" => cur = Some((Event::default(), Pending::default())),
            ("BEGIN", Some(_)) if value == "VALARM" => alarm = Some(Alarm::default()),
            ("BEGIN", Some(_)) => depth_skip = 1,
            ("END", Some((_, p))) if value == "VALARM" => {
                if let Some(a) = alarm.take() {
                    p.alarms.push(a);
                }
            }
            ("END", Some(_)) if value == "VEVENT" => {
                let (e, p) = cur.take().unwrap();
                if e.start != chrono::DateTime::UNIX_EPOCH.naive_utc() {
                    finish(e, p, &mut out, &mut overrides);
                }
            }
            (_, Some(_)) if alarm.is_some() => {
                let a = alarm.as_mut().unwrap();
                match prop.as_str() {
                    "TRIGGER" => {
                        if name.to_uppercase().contains("VALUE=DATE-TIME") {
                            a.absolute = parse_dt(name, value).map(|(d, _, tz)| (d, tz));
                        } else {
                            a.relative = parse_trigger(value);
                            a.related_end = name.to_uppercase().contains("RELATED=END");
                        }
                    }
                    "REPEAT" => a.repeat = value.trim().parse().unwrap_or(0),
                    "DURATION" => a.every = parse_trigger(value).map_or(0, |s| -s),
                    _ => {}
                }
            }
            ("UID", Some((e, _))) => e.uid = value.trim().to_string(),
            ("SUMMARY", Some((e, _))) => e.summary = unescape(value),
            ("LOCATION", Some((e, _))) => e.place = unescape(value),
            ("DESCRIPTION", Some((e, _))) => e.note = unescape(value),
            ("DTSTART", Some((e, _))) => {
                if let Some((d, all, tz)) = parse_dt(name, value) {
                    (e.start, e.all_day, e.tzid) = (d, all, tz);
                }
            }
            ("DTEND", Some((_, p))) => p.end = parse_dt(name, value).map(|(d, _, tz)| (d, tz)),
            ("DURATION", Some((_, p))) => p.duration = parse_trigger(value).map(|s| -s),
            ("RRULE", Some((_, p))) => p.rrule = Some(value.to_string()),
            ("RECURRENCE-ID", Some((_, p))) => p.recurrence_id = parse_dt(name, value).map(|(d, _, tz)| (d, tz)),
            ("EXDATE", Some((_, p))) => p.exdates.extend(value.split(',').filter_map(|v| parse_dt(name, v).map(|(d, _, tz)| (d, tz)))),
            ("RDATE", Some((_, p))) => p.rdates.extend(value.split(',').filter(|v| !v.contains('/')).filter_map(|v| parse_dt(name, v).map(|(d, _, tz)| (d, tz)))),
            (other, Some((e, _))) if !KNOWN.contains(&other) && other != "BEGIN" && other != "END" => e.extra.push(line.clone()),
            _ => {}
        }
    }
    for (uid, when, ev) in overrides {
        // the series skips the occurrence that was changed; the changed one stays an event of its own
        if let Some(master) = out.iter_mut().find(|m| m.uid == uid && m.rule.is_some()) {
            let skip = if master.all_day { when } else { zone::convert(when, ev.tzid.as_deref(), master.tzid.as_deref()) };
            master.skip.push(skip);
        }
        out.push(ev);
    }
    out
}

// ───── recurrence ─────

/// First day of the period (day, week, month or year) that holds `d`
fn period_start(freq: Freq, d: NaiveDate, wkst: Weekday) -> NaiveDate {
    match freq {
        Freq::Daily => d,
        Freq::Weekly => d - Duration::days(i64::from((d.weekday().num_days_from_monday() + 7 - wkst.num_days_from_monday()) % 7)),
        Freq::Monthly => d.with_day(1).unwrap_or(d),
        Freq::Yearly => NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap_or(d),
    }
}

/// The period `n` periods after `base` (a first day of a period)
fn add_periods(freq: Freq, base: NaiveDate, n: i64) -> Option<NaiveDate> {
    match freq {
        Freq::Daily => base.checked_add_signed(Duration::days(n)),
        Freq::Weekly => base.checked_add_signed(Duration::weeks(n)),
        Freq::Monthly | Freq::Yearly => {
            let months = if freq == Freq::Monthly { n } else { n * 12 };
            let total = i64::from(base.year()) * 12 + i64::from(base.month0()) + months;
            NaiveDate::from_ymd_opt(i32::try_from(total.div_euclid(12)).ok()?, u32::try_from(total.rem_euclid(12)).ok()? + 1, 1)
        }
    }
}

/// How many periods lie between two first days of periods
fn periods_between(freq: Freq, a: NaiveDate, b: NaiveDate) -> i64 {
    match freq {
        Freq::Daily => (b - a).num_days(),
        Freq::Weekly => (b - a).num_days().div_euclid(7),
        Freq::Monthly => (i64::from(b.year()) - i64::from(a.year())) * 12 + i64::from(b.month()) - i64::from(a.month()),
        Freq::Yearly => i64::from(b.year()) - i64::from(a.year()),
    }
}

fn days_in_month(y: i32, m: u32) -> u32 {
    let next = if m == 12 { NaiveDate::from_ymd_opt(y + 1, 1, 1) } else { NaiveDate::from_ymd_opt(y, m + 1, 1) };
    next.and_then(|n| n.pred_opt()).map_or(30, |d| d.day())
}

/// Does `d` match one entry of BYDAY, the ordinal counted in the month (or in the year when `in_year`)
fn weekday_matches(d: NaiveDate, by_day: &[(i32, Weekday)], in_year: bool) -> bool {
    by_day.iter().any(|(n, w)| {
        if d.weekday() != *w {
            return false;
        }
        if *n == 0 {
            return true;
        }
        let (idx, total) = if in_year {
            let last = NaiveDate::from_ymd_opt(d.year(), 12, 31).map_or(365, |l| l.ordinal());
            ((d.ordinal() - 1) / 7 + 1, (last - d.ordinal()) / 7 + 1)
        } else {
            ((d.day() - 1) / 7 + 1, (days_in_month(d.year(), d.month()) - d.day()) / 7 + 1)
        };
        if *n > 0 { i64::from(idx) == i64::from(*n) } else { i64::from(total) == i64::from(-*n) }
    })
}

fn monthday_matches(d: NaiveDate, list: &[i32]) -> bool {
    let from_end = i32::try_from(days_in_month(d.year(), d.month()) - d.day()).unwrap_or(0) + 1;
    let day = i32::try_from(d.day()).unwrap_or(0);
    list.iter().any(|n| *n == day || -*n == from_end)
}

/// The dates (sorted) of one period that the rule allows, before BYSETPOS
fn candidates(rule: &Rule, start: NaiveDate, period: NaiveDate) -> Vec<NaiveDate> {
    let month_ok = |d: NaiveDate| rule.by_month.is_empty() || rule.by_month.contains(&d.month());
    let mut days: Vec<NaiveDate> = Vec::new();
    match rule.freq {
        Freq::Daily => days.push(period),
        Freq::Weekly => days.extend((0..7).filter_map(|i| period.checked_add_signed(Duration::days(i)))),
        Freq::Monthly | Freq::Yearly => {
            let year_wide = rule.freq == Freq::Yearly && rule.by_month.is_empty() && rule.by_monthday.is_empty() && !rule.by_day.is_empty();
            let months: Vec<u32> = if rule.freq == Freq::Monthly {
                vec![period.month()]
            } else if !rule.by_month.is_empty() {
                rule.by_month.clone()
            } else if rule.by_monthday.is_empty() && rule.by_day.is_empty() {
                vec![start.month()]
            } else {
                (1..=12).collect()
            };
            for m in months.into_iter().filter(|m| (1..=12).contains(m)) {
                days.extend((1..=days_in_month(period.year(), m)).filter_map(|d| NaiveDate::from_ymd_opt(period.year(), m, d)));
            }
            // No BYMONTHDAY and no BYDAY: the day of the start (a month that has no such day gives nothing)
            if rule.by_monthday.is_empty() && rule.by_day.is_empty() {
                days.retain(|d| d.day() == start.day());
            } else {
                days.retain(|d| (rule.by_monthday.is_empty() || monthday_matches(*d, &rule.by_monthday)) && (rule.by_day.is_empty() || weekday_matches(*d, &rule.by_day, year_wide)));
            }
        }
    }
    if matches!(rule.freq, Freq::Daily | Freq::Weekly) {
        let weekly_default = [(0, start.weekday())];
        let by_day: &[(i32, Weekday)] = if rule.freq == Freq::Weekly && rule.by_day.is_empty() { &weekly_default } else { &rule.by_day };
        days.retain(|d| (by_day.is_empty() || by_day.iter().any(|(_, w)| d.weekday() == *w)) && (rule.by_monthday.is_empty() || monthday_matches(*d, &rule.by_monthday)));
    }
    days.retain(|d| month_ok(*d));
    days.sort();
    days.dedup();
    if !rule.by_setpos.is_empty() {
        let n = i64::try_from(days.len()).unwrap_or(0);
        let picked: Vec<NaiveDate> = rule
            .by_setpos
            .iter()
            .filter_map(|p| {
                let i = if *p > 0 { i64::from(*p) - 1 } else { n + i64::from(*p) };
                usize::try_from(i).ok().filter(|_| i >= 0 && i < n).map(|i| days[i])
            })
            .collect();
        days = picked;
        days.sort();
        days.dedup();
    }
    days
}

/// Walks the series in the zone of the event: the wall times of the occurrences that may touch [from, to), in order
fn series(e: &Event, rule: &Rule, from: NaiveDateTime, to: NaiveDateTime) -> Vec<NaiveDateTime> {
    let start_day = e.start.date();
    let time = e.start.time();
    let first = period_start(rule.freq, start_day, rule.wkst);
    let interval = i64::from(rule.interval);
    // Without COUNT the walk starts near the window; with COUNT it must count from the first occurrence
    let mut p = if rule.count.is_none() {
        let near = period_start(rule.freq, (from - e.length()).date(), rule.wkst);
        (periods_between(rule.freq, first, near).div_euclid(interval) - 1).max(0)
    } else {
        0
    };
    let mut out = Vec::new();
    let mut seen = u32::from(p == 0);
    // The start is the first occurrence, even when the rule would not give it (RFC 5545 section 3.8.5.3)
    if p == 0 && e.start < to && rule.until.is_none_or(|u| e.start <= u) && rule.count.is_none_or(|c| c > 0) {
        out.push(e.start);
    }
    for _ in 0..200_000 {
        let Some(base) = add_periods(rule.freq, first, p * interval) else { break };
        if base > to.date() + Duration::days(1) {
            break;
        }
        for d in candidates(rule, start_day, base) {
            let s = d.and_time(time);
            if s <= e.start && p == 0 || (s == e.start) {
                continue;
            }
            if rule.count.is_some_and(|c| seen >= c) || rule.until.is_some_and(|u| s > u) {
                return out;
            }
            seen += 1;
            if s < to {
                out.push(s);
            }
        }
        p += 1;
    }
    out
}

/// Starts of the occurrences that overlap [from, to), as the views show them (system zone): one that has not ended by
/// `from`, or starts in the window. The series itself is walked in the zone of the event.
pub fn occurrences(e: &Event, from: NaiveDateTime, to: NaiveDateTime) -> Vec<NaiveDateTime> {
    let len = e.length();
    let shown = |s: NaiveDateTime| e.display(s);
    let visible = |s: NaiveDateTime| {
        let d = shown(s);
        d < to && (d >= from || d + len > from) && !e.skip.contains(&s)
    };
    // The window in the zone of the event, widened by a day on each side (zones differ by up to a day)
    let (wall_from, wall_to) = (e.from_display(from) - Duration::days(1), e.from_display(to) + Duration::days(1));
    let mut walls: Vec<NaiveDateTime> = match &e.rule {
        None => vec![e.start],
        Some(rule) => series(e, rule, wall_from, wall_to),
    };
    walls.extend(e.rdate.iter().copied());
    walls.sort();
    walls.dedup();
    walls.into_iter().filter(|s| visible(*s)).map(shown).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }
    fn ev() -> Event {
        Event { uid: "u1@filigrane".into(), summary: "Dentist, 2nd floor; bring card".into(), place: "Rue 1".into(), note: "line1\nline2".into(), start: dt("2026-11-25T14:00"), end: Some(dt("2026-11-25T15:00")), alarms: vec![600, 0], ..Event::default() }
    }
    #[test]
    fn round_trip() {
        let mut e = ev();
        e.rule = Some(Rule::simple(Freq::Weekly, 2, Some(5), None));
        e.skip = vec![dt("2026-12-09T14:00")];
        let text = to_ics(&e);
        assert!(text.contains("TRIGGER:-PT10M") && text.contains("TRIGGER:PT0S"));
        assert_eq!(parse_ics(&text), vec![e]);
    }
    #[test]
    fn long_lines_fold_and_unfold() {
        let mut e = ev();
        e.summary = "é".repeat(100);
        let text = to_ics(&e);
        assert!(text.lines().all(|l| l.len() <= 75));
        assert_eq!(parse_ics(&text)[0].summary, e.summary);
    }
    #[test]
    fn all_day_and_utc() {
        let e = parse_ics("BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261224\r\nSUMMARY:Eve\r\nEND:VEVENT\r\n");
        assert_eq!((e[0].all_day, e[0].start), (true, dt("2026-12-24T00:00")));
        let u = parse_ics("BEGIN:VEVENT\r\nUID:b\r\nDTSTART:20261224T120000Z\r\nSUMMARY:x\r\nEND:VEVENT\r\n");
        // a UTC time keeps its zone; the views show it in the zone of the system
        assert_eq!((u[0].start, u[0].tzid.as_deref()), (dt("2026-12-24T12:00"), Some("UTC")));
        assert_eq!(u[0].display_start(), crate::when::from_epoch(Utc.with_ymd_and_hms(2026, 12, 24, 12, 0, 0).unwrap().timestamp()));
    }
    #[test]
    fn triggers() {
        assert_eq!(parse_trigger("-PT10M"), Some(600));
        assert_eq!(parse_trigger("PT0S"), Some(0));
        assert_eq!(parse_trigger("-P1DT2H"), Some(93600));
        assert_eq!(parse_trigger("PT5M"), Some(-300));
        assert_eq!(fmt_trigger(93600), "-P1DT2H");
        assert_eq!(fmt_trigger(-300), "PT5M");
    }
    #[test]
    fn single_event_window() {
        let e = ev();
        assert_eq!(occurrences(&e, dt("2026-11-25T00:00"), dt("2026-11-26T00:00")), vec![e.start]);
        assert!(occurrences(&e, dt("2026-11-26T00:00"), dt("2026-11-27T00:00")).is_empty());
        // still running at 14:30
        assert_eq!(occurrences(&e, dt("2026-11-25T14:30"), dt("2026-11-25T16:00")), vec![e.start]);
    }
    #[test]
    fn weekly_series_with_skip_and_count() {
        let mut e = ev();
        e.rule = Some(Rule::simple(Freq::Weekly, 1, Some(4), None));
        e.skip = vec![dt("2026-12-02T14:00")];
        let got = occurrences(&e, dt("2026-11-01T00:00"), dt("2027-01-01T00:00"));
        assert_eq!(got, vec![dt("2026-11-25T14:00"), dt("2026-12-09T14:00"), dt("2026-12-16T14:00")]);
    }
    #[test]
    fn daily_series_far_in_the_past_jumps() {
        let mut e = ev();
        e.start = dt("2020-01-01T08:00");
        e.end = None;
        e.rule = Some(Rule::simple(Freq::Daily, 1, None, None));
        let got = occurrences(&e, dt("2026-11-25T00:00"), dt("2026-11-28T00:00"));
        assert_eq!(got, vec![dt("2026-11-25T08:00"), dt("2026-11-26T08:00"), dt("2026-11-27T08:00")]);
    }
    #[test]
    fn monthly_on_the_31st_skips_short_months_and_until_stops() {
        let mut e = ev();
        e.start = dt("2026-01-31T09:00");
        e.end = None;
        e.rule = Some(Rule::simple(Freq::Monthly, 1, None, Some(dt("2026-05-31T09:00"))));
        let got = occurrences(&e, dt("2026-01-01T00:00"), dt("2027-01-01T00:00"));
        assert_eq!(got, vec![dt("2026-01-31T09:00"), dt("2026-03-31T09:00"), dt("2026-05-31T09:00")]);
    }
    #[test]
    fn yearly() {
        let mut e = ev();
        e.start = dt("2024-02-29T00:00");
        e.end = None;
        e.all_day = true;
        e.rule = Some(Rule::simple(Freq::Yearly, 1, None, None));
        let got = occurrences(&e, dt("2024-01-01T00:00"), dt("2029-01-01T00:00"));
        assert_eq!(got, vec![dt("2024-02-29T00:00"), dt("2028-02-29T00:00")]);
    }

    fn series_of(rrule: &str, start: &str, from: &str, to: &str) -> Vec<String> {
        let text = format!("BEGIN:VEVENT\r\nUID:s\r\nDTSTART:{start}\r\nSUMMARY:x\r\nRRULE:{rrule}\r\nEND:VEVENT\r\n");
        let e = parse_ics(&text).remove(0);
        occurrences(&e, dt(from), dt(to)).iter().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).collect()
    }
    #[test]
    fn weekly_on_two_days() {
        // RFC 5545 3.8.5.3: every Monday and Thursday, 10:00; 2026-11-23 is a Monday
        let got = series_of("FREQ=WEEKLY;BYDAY=MO,TH;COUNT=5", "20261123T100000", "2026-11-01T00:00", "2027-01-01T00:00");
        assert_eq!(got, ["2026-11-23 10:00", "2026-11-26 10:00", "2026-11-30 10:00", "2026-12-03 10:00", "2026-12-07 10:00"]);
    }
    #[test]
    fn every_other_week_on_tuesday_and_sunday_depends_on_wkst() {
        // RFC 5545 3.3.10: DTSTART Tuesday 1997-08-05, FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU
        let mo = series_of("FREQ=WEEKLY;INTERVAL=2;COUNT=4;WKST=MO;BYDAY=TU,SU", "19970805T090000", "1997-01-01T00:00", "1998-01-01T00:00");
        assert_eq!(mo, ["1997-08-05 09:00", "1997-08-10 09:00", "1997-08-19 09:00", "1997-08-24 09:00"]);
        let su = series_of("FREQ=WEEKLY;INTERVAL=2;COUNT=4;WKST=SU;BYDAY=TU,SU", "19970805T090000", "1997-01-01T00:00", "1998-01-01T00:00");
        assert_eq!(su, ["1997-08-05 09:00", "1997-08-17 09:00", "1997-08-19 09:00", "1997-08-31 09:00"]);
    }
    #[test]
    fn second_tuesday_and_last_friday_of_the_month() {
        let got = series_of("FREQ=MONTHLY;BYDAY=2TU;COUNT=3", "20261110T090000", "2026-01-01T00:00", "2027-12-31T00:00");
        assert_eq!(got, ["2026-11-10 09:00", "2026-12-08 09:00", "2027-01-12 09:00"]);
        let last = series_of("FREQ=MONTHLY;BYDAY=-1FR;COUNT=3", "20261127T090000", "2026-01-01T00:00", "2027-12-31T00:00");
        assert_eq!(last, ["2026-11-27 09:00", "2026-12-25 09:00", "2027-01-29 09:00"]);
    }
    #[test]
    fn last_day_of_the_month_and_last_weekday_by_setpos() {
        let got = series_of("FREQ=MONTHLY;BYMONTHDAY=-1;COUNT=3", "20261130T090000", "2026-01-01T00:00", "2027-12-31T00:00");
        assert_eq!(got, ["2026-11-30 09:00", "2026-12-31 09:00", "2027-01-31 09:00"]);
        // the last working day of the month (RFC 5545 3.8.5.3)
        let work = series_of("FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1;COUNT=3", "20261130T090000", "2026-01-01T00:00", "2027-12-31T00:00");
        assert_eq!(work, ["2026-11-30 09:00", "2026-12-31 09:00", "2027-01-29 09:00"]);
    }
    #[test]
    fn every_weekday_and_yearly_in_given_months() {
        let got = series_of("FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;COUNT=4", "20261127T090000", "2026-01-01T00:00", "2027-12-31T00:00");
        assert_eq!(got, ["2026-11-27 09:00", "2026-11-30 09:00", "2026-12-01 09:00", "2026-12-02 09:00"]);
        // every year in June and July (RFC 5545 3.8.5.3 example with BYMONTH)
        let got = series_of("FREQ=YEARLY;BYMONTH=6,7;COUNT=4", "19970610T090000", "1997-01-01T00:00", "2000-01-01T00:00");
        assert_eq!(got, ["1997-06-10 09:00", "1997-07-10 09:00", "1998-06-10 09:00", "1998-07-10 09:00"]);
    }
    #[test]
    fn a_rule_that_does_not_give_the_start_still_starts_there() {
        // DTSTART is a Tuesday, the rule says Mondays: the start is the first occurrence anyway
        let got = series_of("FREQ=WEEKLY;BYDAY=MO;COUNT=3", "20261124T090000", "2026-11-01T00:00", "2027-01-01T00:00");
        assert_eq!(got, ["2026-11-24 09:00", "2026-11-30 09:00", "2026-12-07 09:00"]);
    }
    #[test]
    fn unknown_rule_parts_and_properties_are_written_back() {
        let text = "BEGIN:VEVENT\r\nUID:k\r\nDTSTART:20261123T100000\r\nSUMMARY:x\r\nRRULE:FREQ=YEARLY;BYYEARDAY=100\r\nATTENDEE;CN=A:mailto:a@example.org\r\nX-KEEP:1\r\nCATEGORIES:Work\r\nEND:VEVENT\r\n";
        let e = parse_ics(text).remove(0);
        assert!(e.rule.as_ref().unwrap().is_custom(e.start));
        let back = to_ics(&e);
        assert!(back.contains("BYYEARDAY=100") && back.contains("ATTENDEE;CN=A:mailto:a@example.org") && back.contains("X-KEEP:1") && back.contains("CATEGORIES:Work"));
        assert_eq!(parse_ics(&back).remove(0).extra, e.extra);
    }
    #[test]
    fn rdate_and_whole_day_span_and_duration() {
        let e = parse_ics("BEGIN:VEVENT\r\nUID:r\r\nDTSTART:20261123T100000\r\nDURATION:PT90M\r\nSUMMARY:x\r\nRDATE:20261201T100000\r\nEND:VEVENT\r\n").remove(0);
        assert_eq!(e.end, Some(dt("2026-11-23T11:30")));
        assert_eq!(occurrences(&e, dt("2026-11-01T00:00"), dt("2027-01-01T00:00")), vec![dt("2026-11-23T10:00"), dt("2026-12-01T10:00")]);
        let w = parse_ics("BEGIN:VEVENT\r\nUID:w\r\nDTSTART;VALUE=DATE:20261224\r\nDTEND;VALUE=DATE:20261228\r\nSUMMARY:Holidays\r\nEND:VEVENT\r\n").remove(0);
        // 24th to 27th, the 28th is not part of it
        assert_eq!(occurrences(&w, dt("2026-12-27T00:00"), dt("2026-12-28T00:00")).len(), 1);
        assert!(occurrences(&w, dt("2026-12-28T00:00"), dt("2026-12-29T00:00")).is_empty());
    }
    #[test]
    fn alarms_absolute_repeated_and_related_to_the_end() {
        let e = parse_ics("BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20261123T100000Z\r\nDTEND:20261123T110000Z\r\nSUMMARY:x\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;VALUE=DATE-TIME:20261123T090000Z\r\nEND:VALARM\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT10M\r\nREPEAT:2\r\nDURATION:PT5M\r\nEND:VALARM\r\nBEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=END:PT0S\r\nEND:VALARM\r\nEND:VEVENT\r\n").remove(0);
        // an hour before, 10 / 5 / 0 minutes before, and at the end (an hour after the start)
        let mut a = e.alarms.clone();
        a.sort_unstable();
        assert_eq!(a, vec![-3600, 0, 300, 600, 3600]);
    }
    #[test]
    fn a_changed_occurrence_becomes_its_own_event_and_the_series_skips_it() {
        let text = "BEGIN:VEVENT\r\nUID:m\r\nDTSTART:20261123T100000\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\nSUMMARY:Class\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:m\r\nRECURRENCE-ID:20261130T100000\r\nDTSTART:20261130T140000\r\nSUMMARY:Class (moved)\r\nEND:VEVENT\r\n";
        let all = parse_ics(text);
        assert_eq!(all.len(), 2);
        let mut seen: Vec<(String, String)> = all.iter().flat_map(|e| occurrences(e, dt("2026-11-01T00:00"), dt("2027-01-01T00:00")).into_iter().map(move |o| (e.summary.clone(), o.format("%m-%d %H:%M").to_string()))).collect();
        seen.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(seen, [("Class".into(), "11-23 10:00".into()), ("Class (moved)".into(), "11-30 14:00".into()), ("Class".into(), "12-07 10:00".into())]);
    }
    #[test]
    fn a_zoned_series_keeps_its_hour_across_daylight_saving() {
        if !zone::known("Europe/Paris") {
            return;
        }
        // 2026-10-24 is a Saturday; Paris leaves summer time on 2026-10-25, so the same 09:00 wall hour is 07:00 UTC then 08:00 UTC
        let text = "BEGIN:VEVENT\r\nUID:z\r\nDTSTART;TZID=Europe/Paris:20261024T090000\r\nRRULE:FREQ=DAILY;COUNT=3\r\nSUMMARY:x\r\nEND:VEVENT\r\n";
        let e = parse_ics(text).remove(0);
        assert_eq!(e.tzid.as_deref(), Some("Europe/Paris"));
        assert_eq!(e.start, dt("2026-10-24T09:00"));
        let epochs: Vec<i64> = occurrences(&e, dt("2026-10-20T00:00"), dt("2026-10-30T00:00")).iter().map(|d| crate::when::to_epoch(*d)).collect();
        let utc = |s: &str| Utc.from_utc_datetime(&dt(s)).timestamp();
        assert_eq!(epochs, vec![utc("2026-10-24T07:00"), utc("2026-10-25T08:00"), utc("2026-10-26T08:00")]);
        assert!(to_ics(&e).contains("DTSTART;TZID=Europe/Paris:20261024T090000"));
    }
}

#[cfg(test)]
mod fixtures {
    use super::*;

    fn read(name: &str) -> Vec<Event> {
        let path = format!("{}/tests/fixtures/{name}.ics", env!("CARGO_MANIFEST_DIR"));
        parse_ics(&std::fs::read_to_string(path).unwrap())
    }
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }
    /// Wall times in the zone of the event of the occurrences in a window given as wall times of that zone
    fn when(e: &Event, from: &str, to: &str) -> Vec<String> {
        occurrences(e, e.display(dt(from)), e.display(dt(to))).iter().map(|d| e.from_display(*d).format("%m-%d %H:%M").to_string()).collect()
    }

    /// Real calendars (anonymised, written the way each application writes them) are read, written and read again without loss
    #[test]
    fn every_fixture_survives_a_write_and_a_read() {
        for name in ["google", "apple", "proton", "thunderbird"] {
            let events = read(name);
            assert!(!events.is_empty(), "{name}: nothing read");
            for e in &events {
                let again = parse_ics(&to_ics(e));
                assert_eq!(again.len(), 1, "{name}: {}", e.summary);
                let mut a = again.into_iter().next().unwrap();
                let mut b = e.clone();
                // the order of alarms and exceptions is not part of the meaning
                for x in [&mut a, &mut b] {
                    x.alarms.sort_unstable();
                    x.skip.sort();
                    x.rdate.sort();
                }
                assert_eq!(a, b, "{name}: {}", e.summary);
            }
        }
    }

    #[test]
    fn google_weekly_on_two_days_with_a_moved_occurrence() {
        let all = read("google");
        // the series, the moved occurrence and the holidays: three events
        assert_eq!(all.len(), 3);
        let series = all.iter().find(|e| e.summary == "Team sync").unwrap();
        assert_eq!(series.tzid.as_deref(), Some("Europe/Paris"));
        assert_eq!(series.alarms, vec![1800]);
        assert!(series.extra.iter().any(|l| l.starts_with("ATTENDEE;") && l.contains("alex@example.org")));
        // Mondays and Thursdays at 18:00, except the Monday that was moved
        assert_eq!(when(series, "2026-11-22T00:00", "2026-12-08T00:00"), ["11-23 18:00", "11-26 18:00", "12-03 18:00", "12-07 18:00"]);
        let moved = all.iter().find(|e| e.summary == "Team sync (later)").unwrap();
        assert_eq!(when(moved, "2026-11-22T00:00", "2026-12-08T00:00"), ["11-30 20:00"]);
        let holidays = all.iter().find(|e| e.summary == "Holidays").unwrap();
        assert_eq!((holidays.all_day, holidays.end), (true, Some(dt("2026-12-28T00:00"))));
    }

    #[test]
    fn apple_alarms_and_first_thursday() {
        let all = read("apple");
        let call = all.iter().find(|e| e.summary.starts_with("Call")).unwrap();
        // 14:00 New York is 19:00 UTC in December: the absolute alarm at 17:00 UTC is two hours before; related to the end it is
        // 5 minutes before 15:00, which is 55 minutes after the start
        let mut alarms = call.alarms.clone();
        alarms.sort_unstable();
        assert_eq!(alarms, vec![-3300, 7200]);
        assert_eq!(when(call, "2026-12-01T00:00", "2027-03-01T00:00"), ["12-03 14:00", "01-07 14:00", "02-04 14:00"]);
        let year = all.iter().find(|e| e.summary == "New Year").unwrap();
        assert_eq!(when(year, "2027-01-01T00:00", "2029-01-01T00:00"), ["01-01 00:00", "01-01 00:00"]);
    }

    #[test]
    fn proton_utc_series_with_an_exception_and_a_count() {
        let all = read("proton");
        let e = &all[0];
        assert_eq!(e.tzid.as_deref(), Some("UTC"));
        // COUNT=3 wins over the later UNTIL; the second day is deleted
        assert_eq!(when(e, "2026-11-30T00:00", "2027-01-05T00:00"), ["12-01 08:30", "12-03 08:30"]);
    }

    #[test]
    fn thunderbird_keeps_what_it_does_not_use() {
        let e = &read("thunderbird")[0];
        assert!(e.tzid.is_none());
        assert_eq!(e.rdate, vec![dt("2026-12-16T19:00")]);
        for kept in ["CATEGORIES:School,Family", "ORGANIZER;CN=Sam:mailto:sam@example.org", "URL:https://school.example.org/evening", "CLASS:PUBLIC"] {
            assert!(e.extra.iter().any(|l| l == kept), "lost {kept}");
        }
        assert!(e.extra.iter().any(|l| l.starts_with("ATTENDEE;") && l.contains("long@example.org")));
        // 15 minutes before, repeated once 5 minutes later
        let mut a = e.alarms.clone();
        a.sort_unstable();
        assert_eq!(a, vec![600, 900]);
    }
}
