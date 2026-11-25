//! Time zones of events. An event keeps the zone of its file (`TZID`, or UTC for a `Z` time) and its wall time in that zone,
//! so a recurrence stays at the same hour across daylight saving time; the views show it in the zone of the system. The zone
//! database is the one of the system (`/usr/share/zoneinfo`, read by jiff). A zone this system does not know stays in the
//! file as it came, and the event is then shown as a floating one (the hour on the wall).
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use jiff::{civil, tz::TimeZone};

pub fn resolve(tzid: &str) -> Option<TimeZone> {
    if tzid.eq_ignore_ascii_case("utc") || tzid == "Z" {
        return Some(TimeZone::UTC);
    }
    TimeZone::get(tzid).ok()
}

/// Is this zone known here (the editor offers only these)
pub fn known(tzid: &str) -> bool {
    resolve(tzid).is_some()
}

/// The wall time of `epoch` in the zone (the system zone when floating or unknown)
pub fn epoch_to_wall(epoch: i64, tzid: Option<&str>) -> NaiveDateTime {
    let Some(tz) = tzid.and_then(resolve) else { return crate::when::from_epoch(epoch) };
    let Ok(ts) = jiff::Timestamp::from_second(epoch) else { return crate::when::from_epoch(epoch) };
    let d = tz.to_datetime(ts);
    NaiveDate::from_ymd_opt(i32::from(d.year()), u32::try_from(d.month()).unwrap_or(1), u32::try_from(d.day()).unwrap_or(1))
        .and_then(|day| day.and_hms_opt(u32::try_from(d.hour()).unwrap_or(0), u32::try_from(d.minute()).unwrap_or(0), u32::try_from(d.second()).unwrap_or(0)))
        .unwrap_or_default()
}

/// The epoch of a wall time in the zone (the system zone when floating or unknown; the later hour when it exists twice
/// is not chosen: the earlier one, as for local times)
pub fn wall_to_epoch(wall: NaiveDateTime, tzid: Option<&str>) -> i64 {
    let Some(tz) = tzid.and_then(resolve) else { return crate::when::to_epoch(wall) };
    let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(minute), Ok(second)) =
        (i16::try_from(wall.year()), i8::try_from(wall.month()), i8::try_from(wall.day()), i8::try_from(wall.hour()), i8::try_from(wall.minute()), i8::try_from(wall.second()))
    else {
        return crate::when::to_epoch(wall);
    };
    let dt = civil::datetime(year, month, day, hour, minute, second, 0);
    // `compatible`: a time in a gap moves forward, an ambiguous one takes the earlier hour
    tz.to_ambiguous_zoned(dt).compatible().map_or_else(|_| crate::when::to_epoch(wall), |z| z.timestamp().as_second())
}

/// A wall time of one zone as the wall time of another (None = the system zone)
pub fn convert(wall: NaiveDateTime, from: Option<&str>, to: Option<&str>) -> NaiveDateTime {
    let same = from == to;
    let floating = |z: Option<&str>| z.is_none_or(|n| !known(n));
    if same || (floating(from) && floating(to)) {
        return wall;
    }
    epoch_to_wall(wall_to_epoch(wall, from), to)
}

/// The wall time shown by the views (the system zone) for a wall time in the zone of the event
pub fn to_display(wall: NaiveDateTime, tzid: Option<&str>) -> NaiveDateTime {
    convert(wall, tzid, None)
}

/// The wall time in the zone of the event for a time shown by the views
pub fn from_display(shown: NaiveDateTime, tzid: Option<&str>) -> NaiveDateTime {
    convert(shown, None, tzid)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }
    #[test]
    fn paris_is_one_hour_ahead_of_utc_in_winter_and_two_in_summer() {
        if !known("Europe/Paris") {
            return;
        }
        assert_eq!(epoch_to_wall(wall_to_epoch(dt("2026-01-15T12:00"), Some("UTC")), Some("Europe/Paris")), dt("2026-01-15T13:00"));
        assert_eq!(epoch_to_wall(wall_to_epoch(dt("2026-07-15T12:00"), Some("UTC")), Some("Europe/Paris")), dt("2026-07-15T14:00"));
    }
    #[test]
    fn unknown_zone_is_floating() {
        assert!(!known("W. Europe Standard Time"));
        assert_eq!(convert(dt("2026-01-15T12:00"), Some("W. Europe Standard Time"), None), dt("2026-01-15T12:00"));
    }
}
