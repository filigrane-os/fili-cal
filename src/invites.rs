//! Invitations (iTIP, RFC 5546): an `.ics` file with `METHOD:REQUEST` that comes from an e-mail (Facteur gives it to Agenda, the
//! handler of `text/calendar`). Accept, Maybe and Decline put the answer in the event and send it to the organizer with
//! `fili-mail send` (a `METHOD:REPLY` file attached, what other calendar programs understand). Nothing is sent without the
//! user pressing the button: the preview of `fili-mail send` (without `--execute`) is what `fili-cal reply --dry-run` prints.
use std::{
    io::Write,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::{calendars, cli, exchange, ics};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Answer {
    Accept,
    Maybe,
    Decline,
}

impl Answer {
    pub fn parse(s: &str) -> Option<Answer> {
        Some(match s.to_lowercase().as_str() {
            "accept" | "accepted" | "yes" => Answer::Accept,
            "maybe" | "tentative" => Answer::Maybe,
            "decline" | "declined" | "no" => Answer::Decline,
            _ => return None,
        })
    }
    pub fn partstat(self) -> &'static str {
        match self {
            Answer::Accept => "ACCEPTED",
            Answer::Maybe => "TENTATIVE",
            Answer::Decline => "DECLINED",
        }
    }
    fn subject(self, title: &str) -> String {
        format!("{}: {title}", match self {
            Answer::Accept => "Accepted",
            Answer::Maybe => "Tentative",
            Answer::Decline => "Declined",
        })
    }
}

/// The `METHOD` of the calendar (`REQUEST` for an invitation)
pub fn method(text: &str) -> String {
    text.lines().take_while(|l| !l.starts_with("BEGIN:VEVENT")).find_map(|l| l.strip_prefix("METHOD:")).unwrap_or("").trim().to_uppercase()
}

/// The address and the name in a line `ORGANIZER;CN=Sam:mailto:sam@example.org`
pub fn person(line: &str) -> (String, String) {
    let lower = line.to_lowercase();
    let email = lower.rfind("mailto:").map_or(String::new(), |i| line[i + 7..].trim().to_string());
    let name = line.split(':').next().unwrap_or("").split(';').skip(1).find_map(|p| p.strip_prefix("CN=").or_else(|| p.strip_prefix("cn="))).map_or(String::new(), |n| n.trim_matches('"').to_string());
    (email, name)
}

fn lines<'a>(e: &'a ics::Event, name: &'a str) -> impl Iterator<Item = &'a String> {
    e.extra.iter().filter(move |l| l.to_uppercase().starts_with(name) && l[name.len()..].starts_with([':', ';']))
}

/// The accounts of Facteur: (id, e-mail address), the active one first
fn mail_accounts() -> Vec<(String, String)> {
    let Ok(out) = Command::new("fili-mail").args(["accounts", "list", "--json"]).stdin(Stdio::null()).stderr(Stdio::null()).output() else { return vec![] };
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    let active = v["activeId"].as_str().unwrap_or("").to_string();
    let mut list: Vec<(String, String)> = v["accounts"].as_array().into_iter().flatten().filter_map(|a| Some((a["id"].as_str()?.to_string(), a["email"].as_str()?.to_string()))).collect();
    list.sort_by_key(|(id, _)| *id != active);
    list
}

/// Which account of Facteur is invited, and how: (account id, address, name as the attendee line has it)
fn me(e: &ics::Event, accounts: &[(String, String)]) -> Option<(String, String, String)> {
    for line in lines(e, "ATTENDEE") {
        let (email, name) = person(line);
        if let Some((id, addr)) = accounts.iter().find(|(_, a)| a.eq_ignore_ascii_case(&email)) {
            return Some((id.clone(), addr.clone(), name));
        }
    }
    // not found among the attendees: the active account answers
    accounts.first().map(|(id, addr)| (id.clone(), addr.clone(), String::new()))
}

/// What the import preview shows of an invitation: {organizer, name, account, title} or null
pub fn describe(text: &str) -> Value {
    if method(text) != "REQUEST" {
        return Value::Null;
    }
    let Some(e) = ics::parse_ics(text).into_iter().next() else { return Value::Null };
    let Some(org) = lines(&e, "ORGANIZER").next() else { return Value::Null };
    let (email, name) = person(org);
    if email.is_empty() {
        return Value::Null;
    }
    let account = me(&e, &mail_accounts()).map(|m| m.1).unwrap_or_default();
    json!({"organizer": email, "name": if name.is_empty() { email.clone() } else { name }, "account": account, "title": e.summary})
}

/// Sets the answer of `email` in the attendee lines (adds the line when it is not there)
pub fn set_partstat(extra: &mut Vec<String>, email: &str, name: &str, partstat: &str) {
    let mut done = false;
    for l in extra.iter_mut() {
        if l.to_uppercase().starts_with("ATTENDEE") && person(l).0.eq_ignore_ascii_case(email) {
            let (head, value) = l.split_once(":").map(|(h, _)| (h.to_string(), l[h.len()..].to_string())).unwrap_or_default();
            let mut params: Vec<String> = head.split(';').filter(|p| !p.to_uppercase().starts_with("PARTSTAT=") && !p.to_uppercase().starts_with("RSVP=")).map(str::to_string).collect();
            params.push(format!("PARTSTAT={partstat}"));
            *l = format!("{}{value}", params.join(";"));
            done = true;
        }
    }
    if !done {
        let cn = if name.is_empty() { String::new() } else { format!(";CN={name}") };
        extra.push(format!("ATTENDEE;PARTSTAT={partstat}{cn}:mailto:{email}"));
    }
}

/// The text of the `METHOD:REPLY` file for an answer
fn reply_ics(e: &ics::Event, email: &str, name: &str, answer: Answer) -> String {
    let own = ics::to_ics(e);
    let keep = |prefix: &str| own.lines().find(|l| l.starts_with(prefix)).map(str::to_string);
    let mut l = vec!["BEGIN:VCALENDAR".to_string(), "VERSION:2.0".into(), "PRODID:-//Filigrane//fili-cal//EN".into(), "METHOD:REPLY".into(), "BEGIN:VEVENT".into(), format!("UID:{}", e.uid)];
    l.extend(keep("DTSTAMP"));
    l.extend(keep("DTSTART"));
    l.extend(keep("DTEND"));
    l.push(format!("SUMMARY:{}", e.summary.replace('\n', " ")));
    if let Some(o) = lines(e, "ORGANIZER").next() {
        l.push(o.clone());
    }
    let cn = if name.is_empty() { String::new() } else { format!(";CN={name}") };
    l.push(format!("ATTENDEE;PARTSTAT={}{cn}:mailto:{email}", answer.partstat()));
    l.push("END:VEVENT".into());
    l.push("END:VCALENDAR".into());
    l.into_iter().map(|x| x + "\r\n").collect()
}

/// The arguments of `fili-mail send` for an answer: (arguments without `--execute`, body)
pub fn mail_command(account: &str, to: &str, title: &str, answer: Answer, attachment: &str, who: &str) -> (Vec<String>, String) {
    let args = ["send", "--account", account, "--to", to, "--subject", &answer.subject(title), "--attach", attachment].map(str::to_string).to_vec();
    let verb = match answer {
        Answer::Accept => "accepted",
        Answer::Maybe => "tentatively accepted",
        Answer::Decline => "declined",
    };
    (args, format!("{who} has {verb} the invitation: {title}\n"))
}

/// Answers an invitation: the event is added to `calendar` when accepted or tentative (marked with the answer), and the reply goes to
/// the organizer. `dry_run` only prints what would be sent. Returns a sentence for the user.
pub fn reply(source: &str, answer: Answer, calendar: &str, dry_run: bool) -> Result<String, String> {
    let text = exchange::read_source(source)?;
    if method(&text) != "REQUEST" {
        return Err(crate::i18n::tr("this file is not an invitation").to_string());
    }
    let mut events = ics::parse_ics(&text);
    let e = events.first().cloned().ok_or_else(|| crate::i18n::tr("there is no event in this file").to_string())?;
    let (email, _) = person(lines(&e, "ORGANIZER").next().ok_or_else(|| crate::i18n::tr("the invitation has no organizer").to_string())?);
    let (account, my_email, my_name) = me(&e, &mail_accounts()).ok_or_else(|| crate::i18n::tr("no mail account to answer from").to_string())?;
    // the file of the reply
    let dir = cli::state().join("fili-cal");
    std::fs::create_dir_all(&dir).map_err(|x| x.to_string())?;
    let file = dir.join(format!("reply-{}.ics", cli::new_id()));
    std::fs::write(&file, reply_ics(&e, &my_email, &my_name, answer)).map_err(|x| x.to_string())?;
    let (mut args, body) = mail_command(&account, &email, &e.summary, answer, &file.display().to_string(), if my_name.is_empty() { &my_email } else { &my_name });
    if dry_run {
        let _ = std::fs::remove_file(&file);
        return Ok(format!("fili-mail {} --execute  (to {email} from {my_email})", args.join(" ")));
    }
    // the event: kept with the answer when it is accepted or tentative
    if answer != Answer::Decline {
        for ev in events.iter_mut() {
            set_partstat(&mut ev.extra, &my_email, &my_name, answer.partstat());
        }
        let cal = calendars::find(if calendar.is_empty() { calendars::DEFAULT } else { calendar }).ok_or_else(|| format!("no calendar named {calendar}"))?;
        for ev in events {
            let id = cli::short_hash(&format!("{}/{}", cal.slug, ev.uid));
            cli::save(&id, &ics::Event { calendar: cal.slug.clone(), ..ev })?;
        }
        crate::undo::commit(&format!("answer {}", e.summary));
        let _ = cli::tick();
    }
    args.push("--execute".into());
    let mut child = Command::new("fili-mail").args(&args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|_| crate::i18n::tr("Facteur (fili-mail) is not installed").to_string())?;
    child.stdin.take().ok_or("no pipe")?.write_all(body.as_bytes()).map_err(|x| x.to_string())?;
    let out = child.wait_with_output().map_err(|x| x.to_string())?;
    let _ = std::fs::remove_file(&file);
    if !out.status.success() {
        return Err(crate::i18n::trf("the reply could not be sent: %1", &[&String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("").trim()]));
    }
    Ok(crate::i18n::trf("Reply sent to %1", &[&email]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVITE: &str = "BEGIN:VCALENDAR\r\nMETHOD:REQUEST\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:inv-1@example.org\r\nDTSTART:20261210T150000Z\r\nDTEND:20261210T160000Z\r\nSUMMARY:Lunch\r\nORGANIZER;CN=Sam Organizer:mailto:sam@example.org\r\nATTENDEE;CUTYPE=INDIVIDUAL;PARTSTAT=NEEDS-ACTION;RSVP=TRUE;CN=Me Myself:mailto:me@example.org\r\nATTENDEE;PARTSTAT=ACCEPTED;CN=Other:mailto:other@example.org\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    #[test]
    fn an_invitation_is_recognised_and_its_people_read() {
        assert_eq!(method(INVITE), "REQUEST");
        assert_eq!(method("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nMETHOD:no\r\n"), "");
        let e = ics::parse_ics(INVITE).remove(0);
        assert_eq!(person(lines(&e, "ORGANIZER").next().unwrap()), ("sam@example.org".to_string(), "Sam Organizer".to_string()));
        let accounts = vec![("a".to_string(), "other@example.org".to_string()), ("b".to_string(), "me@example.org".to_string())];
        assert_eq!(me(&e, &accounts), Some(("b".into(), "me@example.org".into(), "Me Myself".into())));
        // an invitation that is not a REQUEST is not one
        assert!(describe(&INVITE.replace("METHOD:REQUEST", "METHOD:PUBLISH")).is_null());
    }

    #[test]
    fn the_answer_replaces_the_partstat_of_the_attendee_only() {
        let mut e = ics::parse_ics(INVITE).remove(0);
        set_partstat(&mut e.extra, "ME@example.org", "Me Myself", "DECLINED");
        let mine = e.extra.iter().find(|l| l.contains("me@example.org")).unwrap();
        assert!(mine.contains("PARTSTAT=DECLINED") && !mine.contains("NEEDS-ACTION") && !mine.contains("RSVP") && mine.contains("CN=Me Myself"));
        assert!(e.extra.iter().any(|l| l.contains("other@example.org") && l.contains("PARTSTAT=ACCEPTED")));
        // an attendee line is added when there is none
        let mut none = vec![];
        set_partstat(&mut none, "new@example.org", "", "TENTATIVE");
        assert_eq!(none, ["ATTENDEE;PARTSTAT=TENTATIVE:mailto:new@example.org"]);
    }

    #[test]
    fn the_reply_file_and_the_mail_say_what_was_answered() {
        let e = ics::parse_ics(INVITE).remove(0);
        let r = reply_ics(&e, "me@example.org", "Me Myself", Answer::Accept);
        assert!(r.contains("METHOD:REPLY") && r.contains("UID:inv-1@example.org") && r.contains("ATTENDEE;PARTSTAT=ACCEPTED;CN=Me Myself:mailto:me@example.org") && r.contains("ORGANIZER;CN=Sam Organizer:mailto:sam@example.org") && r.contains("DTSTART:20261210T150000Z"));
        let (args, body) = mail_command("b", "sam@example.org", "Lunch", Answer::Decline, "/tmp/r.ics", "Me Myself");
        assert_eq!(args[..8], ["send", "--account", "b", "--to", "sam@example.org", "--subject", "Declined: Lunch", "--attach"]);
        assert!(!args.iter().any(|a| a == "--execute") && body.contains("declined"));
        assert_eq!(Answer::parse("Maybe"), Some(Answer::Maybe));
    }
}
