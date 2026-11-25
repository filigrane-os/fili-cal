//! Video conferences: the link to join a meeting (Teams, Meet, Zoom, Webex…) found in the place, the notes or the properties of an
//! event, so that Agenda can offer "Join" next to it: on the event, in the alert, in the clock card of the bar, and with
//! `fili-cal join` (the meeting that is on now or starts within half an hour).
use crate::ics::Event;

#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    pub url: String,
    pub service: &'static str,
}

/// The service of a link, from its host and, for the services whose invitations hold other links too (the options of the meeting,
/// the help pages), from its path (None: not a meeting link)
fn service_of(url: &str) -> Option<&'static str> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?.split(':').next()?.to_lowercase();
    let path = rest[rest.find('/').unwrap_or(rest.len())..].to_lowercase();
    let has = |parts: &[&str]| parts.iter().any(|p| path.contains(p));
    let is = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    Some(if is("teams.microsoft.com") || is("teams.live.com") || is("teams.microsoft.us") {
        if !has(&["/l/meetup-join/", "/meet/"]) {
            return None;
        }
        "Teams"
    } else if is("meet.google.com") {
        "Meet"
    } else if is("zoom.us") || is("zoomgov.com") || is("zoom.com") {
        if !has(&["/j/", "/my/", "/wc/join/", "/s/"]) {
            return None;
        }
        "Zoom"
    } else if is("webex.com") {
        if !has(&["/meet/", "/join", "/j.php", "mtid="]) {
            return None;
        }
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

/// Every meeting link in a text, in order, each once
pub fn find_all(text: &str) -> Vec<Join> {
    let mut out: Vec<Join> = Vec::new();
    for w in text.split(|c: char| c.is_whitespace() || "<>\"'()[],;".contains(c)).filter(|w| w.starts_with("https://") || w.starts_with("http://")) {
        let url = w.trim_end_matches(['.', ':', '!', '?']);
        if let Some(service) = service_of(url)
            && !out.iter().any(|j| j.url == url)
        {
            out.push(Join { url: url.to_string(), service });
        }
    }
    out
}

/// The first meeting link in a text: words that start with http(s)://, with what surrounds a link in a mail taken off
pub fn find(text: &str) -> Option<Join> {
    text.split(|c: char| c.is_whitespace() || "<>\"'()[],;".contains(c)).filter(|w| w.starts_with("https://") || w.starts_with("http://")).find_map(|w| {
        let url = w.trim_end_matches(['.', ':', '!', '?']);
        service_of(url).map(|service| Join { url: url.to_string(), service })
    })
}

impl Event {
    /// The meetings of the event, one for each service (an invitation often holds two forms of the same Teams link): the first link
    /// of the service. The first of the list is `join()`.
    pub fn joins(&self) -> Vec<Join> {
        let mut out: Vec<Join> = Vec::new();
        for j in self.join_links() {
            if !out.iter().any(|o| o.service == j.service) {
                out.push(j);
            }
        }
        out
    }

    /// Every meeting link of the event (the place, the notes, the properties), each once
    pub fn join_links(&self) -> Vec<Join> {
        let mut out = find_all(&self.place);
        let rest = std::iter::once(self.note.clone()).chain(self.extra.iter().filter(|l| ["URL", "X-GOOGLE-CONFERENCE", "X-MICROSOFT-SKYPETEAMSMEETINGURL", "CONFERENCE"].iter().any(|p| l.to_uppercase().starts_with(p))).filter_map(|l| l.split_once(':').map(|(_, v)| v.to_string())));
        for text in rest {
            for j in find_all(&text) {
                if !out.iter().any(|o| o.url == j.url) {
                    out.push(j);
                }
            }
        }
        out
    }
    /// The link to join the meeting of this event: in the place, then in the notes, then in the properties some programs add
    /// (`URL`, `X-GOOGLE-CONFERENCE`, `X-MICROSOFT-SKYPETEAMSMEETINGURL`)
    pub fn join(&self) -> Option<Join> {
        find(&self.place).or_else(|| find(&self.note)).or_else(|| self.extra.iter().filter(|l| ["URL", "X-GOOGLE-CONFERENCE", "X-MICROSOFT-SKYPETEAMSMEETINGURL", "CONFERENCE"].iter().any(|p| l.to_uppercase().starts_with(p))).find_map(|l| find(l.split_once(':').map_or("", |(_, v)| v))))
    }
}

/// Opens a link with the program of the user (the browser, or the application of the service), then brings that browser to the
/// front: a program that is told to open a link cannot take the focus by itself on Wayland. It waits up to two seconds for that
/// (the callers that must not wait run it in a thread).
pub fn open(url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(crate::i18n::tr("not a link").to_string());
    }
    std::process::Command::new("xdg-open").arg(url).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map_err(|e| e.to_string())?;
    focus_browser();
    Ok(())
}

/// The application id of the default browser ("google-chrome" for `google-chrome.desktop`)
fn browser_app_id() -> Option<String> {
    let out = std::process::Command::new("xdg-mime").args(["query", "default", "x-scheme-handler/https"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().trim_end_matches(".desktop").to_lowercase();
    if id.is_empty() { None } else { Some(id) }
}

/// Does a window of this application id belong to the browser of this desktop id (`org.mozilla.firefox` and `firefox`)
fn same_app(window: &str, browser: &str) -> bool {
    let w = window.to_lowercase();
    w == browser || browser.ends_with(&format!(".{w}")) || w.ends_with(&format!(".{browser}"))
}

/// The window to bring to the front among niri's: (id, focused) of the browser window that was last in front
fn browser_window(windows: &serde_json::Value, browser: &str) -> Option<(u64, bool)> {
    windows
        .as_array()?
        .iter()
        .filter(|w| w["app_id"].as_str().is_some_and(|a| same_app(a, browser)))
        .max_by_key(|w| (w["focus_timestamp"]["secs"].as_u64().unwrap_or(0), w["focus_timestamp"]["nanos"].as_u64().unwrap_or(0)))
        .and_then(|w| Some((w["id"].as_u64()?, w["is_focused"].as_bool().unwrap_or(false))))
}

/// Brings the browser to the front with niri (nothing happens elsewhere): the window that was last in front, once the browser has had
/// the time to take the link, and again if it is not in front a moment later
fn focus_browser() {
    if std::env::var_os("NIRI_SOCKET").is_none() {
        return;
    }
    let Some(browser) = browser_app_id() else { return };
    for wait in [600u64, 900] {
        std::thread::sleep(std::time::Duration::from_millis(wait));
        let Ok(out) = std::process::Command::new("niri").args(["msg", "--json", "windows"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output() else { return };
        let windows: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
        match browser_window(&windows, &browser) {
            Some((_, true)) => return,
            Some((id, false)) => {
                let _ = std::process::Command::new("niri").args(["msg", "action", "focus-window", "--id", &id.to_string()]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
            }
            None => return,
        }
    }
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
    fn an_invitation_with_the_options_page_gives_only_the_meeting_and_a_note_can_hold_several_links() {
        let note = "Join: <https://teams.microsoft.com/l/meetup-join/19%3ameeting_x/0> Meeting options: https://teams.microsoft.com/meetingOptions/?organizerId=1 Help: https://support.zoom.us/hc/en-us Backup https://meet.google.com/aaa-bbbb-ccc";
        let e = Event { note: note.into(), ..Event::default() };
        let all = e.joins();
        assert_eq!(all.iter().map(|j| j.service).collect::<Vec<_>>(), ["Teams", "Meet"]);
        // two forms of the same Teams link are one meeting for the buttons, two links for what is taken out of the notes
        let two = Event { note: "https://teams.microsoft.com/meet/123?p=abc and https://teams.microsoft.com/l/meetup-join/19%3ameeting_x/0".into(), ..Event::default() };
        assert_eq!((two.joins().len(), two.join_links().len()), (1, 2));
        assert_eq!(e.join().map(|j| j.service), Some("Teams"));
        // the same link twice is one button
        let twice = Event { place: "https://meet.google.com/aaa-bbbb-ccc".into(), note: "https://meet.google.com/aaa-bbbb-ccc".into(), ..Event::default() };
        assert_eq!(twice.joins().len(), 1);
    }

    #[test]
    fn the_browser_window_that_was_last_in_front_is_the_one_to_focus() {
        let w = serde_json::json!([
            {"id": 17, "app_id": "google-chrome", "is_focused": false, "focus_timestamp": {"secs": 21642, "nanos": 7}},
            {"id": 44, "app_id": "google-chrome", "is_focused": false, "focus_timestamp": {"secs": 22511, "nanos": 2}},
            {"id": 16, "app_id": "fili-term", "is_focused": true, "focus_timestamp": {"secs": 22600, "nanos": 0}}
        ]);
        assert_eq!(browser_window(&w, "google-chrome"), Some((44, false)));
        assert_eq!(browser_window(&w, "firefox"), None);
        // the desktop id of a flatpak-style browser and the application id of its windows
        assert!(same_app("firefox", "org.mozilla.firefox") && same_app("Google-Chrome", "google-chrome") && !same_app("fili-term", "google-chrome"));
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
