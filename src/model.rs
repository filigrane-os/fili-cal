//! What the window exchanges with the store: events as JSON (the occurrences of a range), and the changes made from the
//! editor. The files are the ones of the command line (`cli`), so the window, `fili-cal` and Lia see the same events.
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::{Value, json};

use crate::{
    cli,
    ics::{self, Event, Freq, Rule},
    calendars, undo, when, zone,
};

const MINUTE: &str = "%Y-%m-%dT%H:%M";

fn parse_day(s: &str) -> Option<NaiveDateTime> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0))
}

fn parse_start(s: &str) -> Result<(NaiveDateTime, bool), String> {
    if let Ok(d) = NaiveDateTime::parse_from_str(s, MINUTE) {
        return Ok((d, false));
    }
    parse_day(s).map(|d| (d, true)).ok_or_else(|| format!("bad date: {s}"))
}

fn one(id: &str, e: &Event, occ: NaiveDateTime, color: &str, readonly: bool) -> Value {
    let end = e.end_of(occ);
    // `start`, `end`, `occ` and `first` are what the views show (system zone); `zstart` and `zend` are the same time in the
    // zone of the event (`tz`, empty = floating), which the editor shows
    let zone_of = |shown: NaiveDateTime| e.from_display(shown).format(MINUTE).to_string();
    json!({
        "id": id, "title": e.summary, "start": occ.format(MINUTE).to_string(),
        "end": end.map(|x| x.format(MINUTE).to_string()), "allDay": e.all_day, "place": e.place, "note": e.note,
        "tz": e.tzid.clone().unwrap_or_default(),
        "calendar": if e.calendar.is_empty() { calendars::DEFAULT } else { &e.calendar }, "color": color, "readonly": readonly, "zstart": zone_of(occ), "zend": end.map(zone_of),
        "alarms": e.alarms, "every": e.rule.as_ref().map_or("", |r| freq_word(r.freq)),
        "interval": e.rule.as_ref().map_or(1, |r| r.interval), "count": e.rule.as_ref().and_then(|r| r.count),
        "until": e.rule.as_ref().and_then(|r| r.until).map(|u| u.format(MINUTE).to_string()),
        "custom": e.rule.as_ref().is_some_and(|r| r.is_custom(e.start)), "rrule": e.rule.as_ref().map_or(String::new(), |r| r.text(e.all_day)),
        "occ": occ.format(MINUTE).to_string(), "first": e.display_start().format(MINUTE).to_string(), "zfirst": e.start.format(MINUTE).to_string(), "series": e.rule.is_some() || !e.rdate.is_empty()
    })
}

fn freq_word(f: Freq) -> &'static str {
    match f {
        Freq::Daily => "day",
        Freq::Weekly => "week",
        Freq::Monthly => "month",
        Freq::Yearly => "year",
    }
}

/// The occurrences between two days (`to` excluded) as a JSON array, in order
#[allow(dead_code)]
pub fn events_json(from: &str, to: &str) -> String {
    events_json_of(&cli::load_all(), from, to)
}

pub fn events_json_of(all: &[(String, Event)], from: &str, to: &str) -> String {
    let (Some(from), Some(to)) = (parse_day(from), parse_day(to)) else { return "[]".into() };
    let mut rows: Vec<(NaiveDateTime, String, Value)> = Vec::new();
    // calendars that are hidden show nothing in the views (their alerts still ring)
    let cals = calendars::list();
    for (id, e) in all {
        let slug = if e.calendar.is_empty() { calendars::DEFAULT } else { &e.calendar };
        let Some(cal) = cals.iter().find(|c| c.slug == slug) else { continue };
        if cal.hidden {
            continue;
        }
        for occ in ics::occurrences(e, from, to) {
            rows.push((occ, e.summary.clone(), one(id, e, occ, &cal.color, cal.url.is_some())));
        }
    }
    rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    Value::Array(rows.into_iter().map(|r| r.2).collect()).to_string()
}

/// The fields of the JSON replace those of the event
fn apply(e: &mut Event, v: &Value) -> Result<(), String> {
    let text = |k: &str| v.get(k).and_then(Value::as_str);
    if let Some(t) = text("title") {
        if t.trim().is_empty() {
            return Err("the title is empty".into());
        }
        e.summary = t.trim().to_string();
    }
    if let Some(c) = text("calendar") {
        e.calendar = calendars::find(c).ok_or_else(|| format!("no calendar named {c}"))?.slug;
    }
    if let Some(p) = text("place") {
        e.place = p.to_string();
    }
    if let Some(n) = text("note") {
        e.note = n.to_string();
    }
    // `tz` present: the times of this request are wall times of that zone ("" = floating); absent: they are what the views
    // show and are brought to the zone of the event
    let typed_in_zone = v.get("tz").is_some();
    if let Some(z) = text("tz") {
        let z = z.trim();
        if z.is_empty() || z.eq_ignore_ascii_case("floating") {
            e.tzid = None;
        } else if zone::known(z) {
            e.tzid = Some(z.to_string());
        } else {
            return Err(format!("unknown time zone: {z}"));
        }
    }
    let was_all_day = e.all_day;
    if let Some(all) = v.get("allDay").and_then(Value::as_bool) {
        e.all_day = all;
    }
    let to_event_zone = |e: &Event, d: NaiveDateTime| if typed_in_zone { d } else { e.from_display(d) };
    if let Some(s) = text("start") {
        let (d, all) = parse_start(s)?;
        let len = e.length();
        e.all_day = v.get("allDay").and_then(Value::as_bool).unwrap_or(all);
        e.start = if e.all_day { d } else { to_event_zone(e, d) };
        if e.all_day != was_all_day {
            e.end = None;
        } else if e.end.is_some() && v.get("end").is_none() {
            e.end = Some(e.start + len);
        }
    } else if e.all_day != was_all_day {
        e.end = None;
    }
    if let Some(end) = v.get("end") {
        e.end = match end.as_str() {
            // a whole-day event ends on the day after its last one (DTEND is exclusive); a single day has no end
            Some(s) if !s.is_empty() => {
                let d = parse_start(s)?.0;
                let d = if e.all_day { d } else { to_event_zone(e, d) };
                if e.all_day && d <= e.start { None } else { Some(d) }
            }
            _ => None,
        };
    }
    if let Some(end) = e.end
        && end < e.start
    {
        return Err("the end is before the start".into());
    }
    if let Some(a) = v.get("alarms").and_then(Value::as_array) {
        e.alarms = a.iter().filter_map(Value::as_i64).collect();
    }
    if let Some(every) = text("every") {
        if every.is_empty() || every == "none" {
            e.rule = None;
        } else {
            let freq = Freq::parse(every).ok_or("every: day, week, month or year")?;
            // the same frequency keeps the rule as it is (its BYDAY… parts); another one starts a simple rule
            let old = e.rule.take();
            e.rule = Some(match old {
                Some(r) if r.freq == freq => r,
                Some(r) => Rule::simple(freq, r.interval, r.count, r.until),
                None => Rule::simple(freq, 1, None, None),
            });
        }
    }
    if let Some(r) = e.rule.as_mut() {
        if let Some(i) = v.get("interval").and_then(Value::as_u64) {
            r.interval = u32::try_from(i.max(1)).unwrap_or(1);
        }
        if let Some(c) = v.get("count") {
            r.count = c.as_u64().filter(|n| *n > 0).and_then(|n| u32::try_from(n).ok());
        }
        if let Some(u) = v.get("until") {
            r.until = u.as_str().filter(|s| !s.is_empty()).map(parse_start).transpose()?.map(|p| p.0);
        }
    }
    Ok(())
}

pub fn add(json: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let id = cli::new_id();
    let now = when::now();
    let mut e = Event { uid: format!("{id}@filigrane"), start: now, ..Event::default() };
    if v.get("title").and_then(Value::as_str).is_none_or(|t| t.trim().is_empty()) {
        return Err("the title is empty".into());
    }
    if v.get("start").is_none() {
        return Err("the start is missing".into());
    }
    apply(&mut e, &v)?;
    if v.get("alarms").is_none() {
        e.alarms = if e.all_day { vec![] } else { crate::settings::default_alarms() };
    }
    cli::save(&id, &e)?;
    undo::commit(&format!("add {}", e.summary));
    let _ = cli::tick();
    Ok(id)
}

pub fn update(id: &str, json: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut e = cli::load(id)?;
    apply(&mut e, &v)?;
    cli::save(id, &e)?;
    undo::commit(&format!("edit {}", e.summary));
    let _ = cli::tick();
    Ok(())
}

/// Deletes the event, or its occurrence starting at `occ` (YYYY-MM-DDTHH:MM, empty = the whole event)
pub fn remove(id: &str, occ: &str) -> Result<(), String> {
    if occ.is_empty() {
        cli::delete(id)?;
        undo::commit(&format!("delete {id}"));
    } else {
        let mut e = cli::load(id)?;
        let (d, _) = parse_start(occ)?;
        if e.rule.is_none() && e.rdate.is_empty() {
            return Err("not a series".into());
        }
        e.skip.push(e.from_display(d));
        cli::save(id, &e)?;
        undo::commit(&format!("delete one occurrence of {}", e.summary));
    }
    let _ = cli::tick();
    Ok(())
}

/// "tomorrow 14:30 Dentist", "+30m Bread", "2026-12-24 Christmas Eve": the start of the line is a date, the rest the title
pub fn parse_quick(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let now = when::now();
    let found = (1..=words.len().min(3)).rev().find_map(|n| {
        let head: Vec<&str> = words[..n].iter().copied().enumerate().filter(|(i, w)| !(*i == 1 && matches!(*w, "à" | "at"))).map(|(_, w)| w).collect();
        when::parse_when(&head.join(" "), now).ok().map(|w| (n, w))
    });
    let Some((n, (start, all_day))) = found else { return "{}".into() };
    let title = words[n..].join(" ");
    if title.is_empty() {
        return "{}".into();
    }
    let reminder = words[0].starts_with('+');
    let alarms: Vec<i64> = if reminder { vec![0] } else if all_day { vec![] } else { crate::settings::default_alarms() };
    let end = if all_day || reminder { None } else { Some(start + Duration::seconds(crate::settings::default_length())) };
    json!({"title": title, "start": if all_day { start.format("%Y-%m-%d").to_string() } else { start.format(MINUTE).to_string() }, "end": end.map(|e| e.format(MINUTE).to_string()), "allDay": all_day, "alarms": alarms, "reminder": reminder}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_reads_a_date_then_a_title() {
        unsafe { std::env::set_var("FILI_CAL_NOW", "2026-11-25T10:00") };
        let v: Value = serde_json::from_str(&parse_quick("demain à 9h Appeler la banque")).unwrap();
        assert_eq!((v["title"].as_str(), v["start"].as_str(), v["allDay"].as_bool()), (Some("Appeler la banque"), Some("2026-11-26T09:00"), Some(false)));
        let v: Value = serde_json::from_str(&parse_quick("+30m Bread")).unwrap();
        assert_eq!((v["reminder"].as_bool(), v["alarms"][0].as_i64()), (Some(true), Some(0)));
        assert_eq!(parse_quick("Dentist"), "{}");
        assert_eq!(parse_quick("14:00"), "{}");
    }
}
