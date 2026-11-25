//! Video conferences: the link to join a meeting (Teams, Meet, Zoom, Webex…) found in the place, the notes or the properties of an
//! event, so that Agenda can offer "Join" next to it: on the event, in the alert, in the clock card of the bar, and with
//! `fili-cal join` (the meeting that is on now or starts within half an hour).
use crate::ics::Event;

#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub url: String,
    pub service: &'static str,
}

/// The service of a link, from its host (None: not a meeting link)
fn service_of(url: &str) -> Option<&'static str> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?.split(':').next()?.to_lowercase();
    let is = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    Some(if is("teams.microsoft.com") || is("teams.live.com") || is("teams.microsoft.us") {
        "Teams"
    } else if is("meet.google.com") {
        "Meet"
    } else if is("zoom.us") || is("zoomgov.com") || is("zoom.com") {
        "Zoom"
    } else if is("webex.com") {
        "Webex"
    } else if is("whereby.com") {
        "Whereby"
    } else if is("meet.jit.si") || host.starts_with("jitsi.") || host.starts_with("meet.") && rest.contains("jitsi") {
        "Jitsi"
    } else if is("gotomeet.me") || is("gotomeeting.com") || is("goto.com") {
        "GoTo"
    } else if is("bluejeans.com") {
        "BlueJeans"
    } else if is("around.co") {
        "Around"
    } else if is("whereby.com") || is("meet.proton.me") {
        "Meeting"
    } else if is("slack.com") && rest.contains("/huddle/") {
        "Slack"
    } else {
        return None;
    })
}

/// The first meeting link in a text: words that start with http(s)://, with what surrounds a link in a mail taken off
pub fn find(text: &str) -> Option<Join> {
    text.split(|c: char| c.is_whitespace() || "<>\"'()[],;".contains(c)).filter(|w| w.starts_with("https://") || w.starts_with("http://")).find_map(|w| {
        let url = w.trim_end_matches(['.', ':', '!', '?']);
        service_of(url).map(|service| Join { url: url.to_string(), service })
    })
}

impl Event {
    /// The link to join the meeting of this event: in the place, then in the notes, then in the properties some programs add
    /// (`URL`, `X-GOOGLE-CONFERENCE`, `X-MICROSOFT-SKYPETEAMSMEETINGURL`)
    pub fn join(&self) -> Option<Join> {
        find(&self.place).or_else(|| find(&self.note)).or_else(|| self.extra.iter().filter(|l| ["URL", "X-GOOGLE-CONFERENCE", "X-MICROSOFT-SKYPETEAMSMEETINGURL", "CONFERENCE"].iter().any(|p| l.to_uppercase().starts_with(p))).find_map(|l| find(l.split_once(':').map_or("", |(_, v)| v))))
    }
}

/// Opens a link with the program of the user (the browser, or the application of the service)
pub fn open(url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(crate::i18n::tr("not a link").to_string());
    }
    std::process::Command::new("xdg-open").arg(url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_of_the_usual_services_are_found_in_text_as_mails_write_it() {
        let teams = "Join Microsoft Teams Meeting <https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc/0?context=%7b%22Tid%22%3a%22x%22%7d>\nMeeting ID: 123 456";
        assert_eq!(find(teams).map(|j| j.service), Some("Teams"));
        assert!(find(teams).unwrap().url.ends_with("%7d"));
        assert_eq!(find("Réunion: https://meet.google.com/abc-defg-hij.").map(|j| (j.url, j.service)), Some(("https://meet.google.com/abc-defg-hij".into(), "Meet")));
        assert_eq!(find("(https://us02web.zoom.us/j/123456789?pwd=abc)").map(|j| j.service), Some("Zoom"));
        assert_eq!(find("https://acme.webex.com/meet/jo").map(|j| j.service), Some("Webex"));
        assert_eq!(find("https://meet.jit.si/Standup").map(|j| j.service), Some("Jitsi"));
        // a link that is not a meeting, a look-alike host, and no link
        assert_eq!(find("Agenda: https://docs.example.org/plan"), None);
        assert_eq!(find("https://notzoom.us.example.org/j/1"), None);
        assert_eq!(find("Room 4, second floor"), None);
    }

    #[test]
    fn an_event_gives_the_link_of_its_place_its_notes_or_its_properties() {
        let mut e = Event { place: "Salle 4".into(), note: "Rejoindre: https://meet.google.com/xyz-abcd-efg".into(), ..Event::default() };
        assert_eq!(e.join().map(|j| j.service), Some("Meet"));
        e.place = "https://us02web.zoom.us/j/1".into();
        assert_eq!(e.join().map(|j| j.service), Some("Zoom"));
        let g = Event { extra: vec!["X-GOOGLE-CONFERENCE:https://meet.google.com/aaa-bbbb-ccc".into()], ..Event::default() };
        assert_eq!(g.join().map(|j| j.url), Some("https://meet.google.com/aaa-bbbb-ccc".into()));
        assert!(open("file:///etc/passwd").is_err());
    }
}
