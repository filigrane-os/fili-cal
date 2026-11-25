//! CalDAV accounts (Nextcloud, Fastmail, iCloud, any server with an app password): a calendar of the account is a folder of the
//! store like any other, and its events are kept in step with the server. Written without a resident process: the sync runs in a
//! process of its own, started at the connection, by "Refresh", and by `fili-cal tick` (slowly, and at once after a change).
//!
//! * HTTP goes through `curl` (no TLS stack here); the password is read from the keyring (`secret-tool`) and handed to curl on its
//!   standard input, never written in a file of the user or of the repository, never on a command line.
//! * A sync first pushes what changed here (new file → PUT, changed → PUT with `If-Match`, deleted → DELETE), then pulls what
//!   changed there (the `ETag` of each resource, then `calendar-multiget`). When both sides changed, the server wins and the local
//!   version is kept beside it in `<id>.conflict`.
//! * An event of the server that is made of several VEVENTs (a series with changed occurrences) is split here into several
//!   events, which could not be written back as one: it is read-only here.
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::{calendars, cli, ics, xml};

pub struct Resp {
    pub status: u16,
    /// The response headers, one `Name: value` per line
    pub headers: String,
    pub body: String,
}

impl Resp {
    pub fn header(&self, name: &str) -> String {
        self.headers.lines().rev().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_string())).unwrap_or_default()
    }
    fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub trait Transport {
    fn request(&self, method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>) -> Result<Resp, String>;
}

// ───── urls ─────

fn host_of(url: &str) -> &str {
    url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or("")
}

/// `scheme://host`
fn origin(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    format!("{scheme}://{}", rest.split('/').next().unwrap_or(""))
}

/// An absolute URL for a `href` of a response (an absolute path, a full URL, or relative to `base`)
pub fn join(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("{}{href}", origin(base))
    } else {
        format!("{}/{href}", base.rsplit_once('/').map_or(base, |(d, _)| d))
    }
}

fn path_of(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    rest.find('/').map_or("/".to_string(), |i| rest[i..].to_string())
}

fn with_slash(url: &str) -> String {
    if url.ends_with('/') { url.to_string() } else { format!("{url}/") }
}

fn same_resource(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

// ───── curl ─────

pub struct Curl {
    user: String,
    password: String,
}

impl Curl {
    pub fn new(user: &str, password: &str) -> Curl {
        Curl { user: user.to_string(), password: password.to_string() }
    }
    fn quote(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"").replace(['\n', '\r'], "")
    }
    fn once(&self, method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>, auth: bool) -> Result<Resp, String> {
        let tmp = cli::state().join("fili-cal");
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let stem = format!("dav-{}-{}", std::process::id(), cli::new_id());
        let (out, head, data) = (tmp.join(format!("{stem}.out")), tmp.join(format!("{stem}.head")), tmp.join(format!("{stem}.data")));
        let mut cfg = format!("url = \"{}\"\nrequest = \"{method}\"\nsilent\nshow-error\nmax-time = 60\nmax-filesize = 30000000\nproto = \"=https,http\"\noutput = \"{}\"\ndump-header = \"{}\"\nwrite-out = \"%{{http_code}}\"\n", Self::quote(url), out.display(), head.display());
        if auth {
            cfg += &format!("user = \"{}:{}\"\n", Self::quote(&self.user), Self::quote(&self.password));
        }
        for (k, v) in headers {
            cfg += &format!("header = \"{}: {}\"\n", Self::quote(k), Self::quote(v));
        }
        if let Some(b) = body {
            fs::write(&data, b).map_err(|e| e.to_string())?;
            cfg += &format!("data-binary = \"@{}\"\n", data.display());
        }
        let mut child = Command::new("curl").args(["--config", "-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|_| crate::i18n::tr("curl is not installed").to_string())?;
        child.stdin.take().ok_or("no pipe")?.write_all(cfg.as_bytes()).map_err(|e| e.to_string())?;
        let result = child.wait_with_output().map_err(|e| e.to_string())?;
        let status: u16 = String::from_utf8_lossy(&result.stdout).trim().parse().unwrap_or(0);
        let resp = Resp { status, headers: fs::read_to_string(&head).unwrap_or_default().replace("\r\n", "\n"), body: String::from_utf8_lossy(&fs::read(&out).unwrap_or_default()).to_string() };
        let _ = (fs::remove_file(&out), fs::remove_file(&head), fs::remove_file(&data));
        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr).lines().next().unwrap_or("curl failed").to_string());
        }
        Ok(resp)
    }
}

impl Transport for Curl {
    /// Redirects are followed here (curl would turn PROPFIND into GET); the password goes only to the host it was given for
    fn request(&self, method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>) -> Result<Resp, String> {
        let mut target = url.to_string();
        for _ in 0..5 {
            let auth = host_of(&target) == host_of(url);
            let r = self.once(method, &target, headers, body, auth)?;
            if matches!(r.status, 301 | 302 | 307 | 308) {
                let loc = r.header("location");
                if loc.is_empty() {
                    return Ok(r);
                }
                target = join(&target, &loc);
                continue;
            }
            return Ok(r);
        }
        Err("too many redirects".into())
    }
}

// ───── keyring ─────

fn label(slug: &str) -> String {
    format!("Agenda: {slug}")
}

pub fn store_password(slug: &str, password: &str) -> Result<(), String> {
    let mut child = Command::new("secret-tool")
        .args(["store", "--label", &label(slug), "service", "fili-cal", "account", slug])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| crate::i18n::tr("the keyring is not available (secret-tool)").to_string())?;
    child.stdin.take().ok_or("no pipe")?.write_all(password.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(crate::i18n::tr("the keyring refused the password").to_string()) }
}

pub fn password(slug: &str) -> Result<String, String> {
    let out = Command::new("secret-tool").args(["lookup", "service", "fili-cal", "account", slug]).stdin(Stdio::null()).stderr(Stdio::null()).output().map_err(|_| crate::i18n::tr("the keyring is not available (secret-tool)").to_string())?;
    let p = String::from_utf8_lossy(&out.stdout).to_string();
    if out.status.success() && !p.is_empty() { Ok(p) } else { Err(crate::i18n::tr("no password in the keyring for this account: connect it again").to_string()) }
}

pub fn forget_password(slug: &str) {
    let _ = Command::new("secret-tool").args(["clear", "service", "fili-cal", "account", slug]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

// ───── discovery ─────

pub struct Collection {
    pub url: String,
    pub name: String,
}

const PROPFIND_HEAD: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:propfind xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\" xmlns:cs=\"http://calendarserver.org/ns/\"><d:prop>";
const XML: (&str, &str) = ("Content-Type", "application/xml; charset=utf-8");

fn propfind(t: &dyn Transport, url: &str, depth: &str, props: &str) -> Result<Resp, String> {
    let body = format!("{PROPFIND_HEAD}{props}</d:prop></d:propfind>");
    let r = t.request("PROPFIND", url, &[XML, ("Depth", depth)], Some(&body))?;
    match r.status {
        207 => Ok(r),
        401 | 403 => Err(crate::i18n::tr("the server refused the user name or the password").to_string()),
        s => Err(crate::i18n::trf("the server answered %1", &[&s])),
    }
}

/// The calendars of an account, from the address the user gave: the server, the principal, the home of the calendars, or one calendar
pub fn discover(t: &dyn Transport, start: &str) -> Result<Vec<Collection>, String> {
    let start = start.trim();
    let mut url = start.to_string();
    let first = propfind(t, &url, "0", "<d:resourcetype/><d:displayname/><d:current-user-principal/><c:calendar-home-set/>").or_else(|e| {
        // a bare server name: the well-known address of CalDAV (RFC 6764)
        if path_of(start) == "/" { propfind(t, &format!("{}/.well-known/caldav", origin(start)), "0", "<d:resourcetype/><d:displayname/><d:current-user-principal/><c:calendar-home-set/>").map_err(|_| e) } else { Err(e) }
    })?;
    if let Some(r) = elements_of(&first.body).first()
        && let Some(rt) = xml::first(r, "resourcetype")
        && xml::has(rt, "calendar")
    {
        let name = xml::first(r, "displayname").map(xml::text).filter(|n| !n.is_empty()).unwrap_or_else(|| last_segment(&url));
        return Ok(vec![Collection { url: with_slash(&url), name }]);
    }
    let href_in = |body: &str, outer: &str| xml::first(body, outer).and_then(|o| xml::first(o, "href")).map(xml::text).filter(|h| !h.is_empty());
    let home = match href_in(&first.body, "calendar-home-set") {
        Some(h) => join(&url, &h),
        None => {
            let principal = href_in(&first.body, "current-user-principal").ok_or_else(|| crate::i18n::tr("no calendars were found at this address").to_string())?;
            url = join(&url, &principal);
            let p = propfind(t, &url, "0", "<c:calendar-home-set/>")?;
            join(&url, &href_in(&p.body, "calendar-home-set").ok_or_else(|| crate::i18n::tr("no calendars were found at this address").to_string())?)
        }
    };
    let list = propfind(t, &home, "1", "<d:resourcetype/><d:displayname/><c:supported-calendar-component-set/>")?;
    let mut out = Vec::new();
    for r in elements_of(&list.body) {
        let (Some(href), Some(rt)) = (xml::first(r, "href"), xml::first(r, "resourcetype")) else { continue };
        if !xml::has(rt, "calendar") {
            continue;
        }
        // calendars that cannot hold events (tasks only) are left out; no information means events
        let comps = xml::attrs_of(r, "comp", "name");
        if !comps.is_empty() && !comps.iter().any(|c| c.eq_ignore_ascii_case("VEVENT")) {
            continue;
        }
        let url = with_slash(&join(&home, &xml::text(href)));
        let name = xml::first(r, "displayname").map(xml::text).filter(|n| !n.is_empty()).unwrap_or_else(|| last_segment(&url));
        out.push(Collection { url, name });
    }
    if out.is_empty() { Err(crate::i18n::tr("no calendars were found at this address").to_string()) } else { Ok(out) }
}

fn last_segment(url: &str) -> String {
    url.trim_end_matches('/').rsplit('/').next().unwrap_or("calendar").to_string()
}

fn elements_of(body: &str) -> Vec<&str> {
    xml::elements(body, "response")
}

/// Connects an account: creates a calendar for each calendar of the server and keeps the password in the keyring.
/// Returns the names of the calendars made.
pub fn connect(t: &dyn Transport, address: &str, user: &str, password: &str, keep: bool) -> Result<Vec<String>, String> {
    let found = discover(t, address)?;
    let mut made = Vec::new();
    for c in found {
        let host = host_of(&c.url).to_string();
        let name = if calendars::find(&c.name).is_some() { format!("{} ({host})", c.name) } else { c.name.clone() };
        let cal = calendars::create_account(&name, &c.url, user)?;
        if keep {
            store_password(&cal.slug, password)?;
        }
        made.push(cal.name);
    }
    Ok(made)
}

// ───── state ─────

#[derive(Clone, Default)]
pub struct Item {
    pub etag: String,
    pub hash: String,
    pub ids: Vec<String>,
    /// One VEVENT: it can be written back
    pub single: bool,
}

#[derive(Default)]
pub struct DavState {
    pub ctag: String,
    pub items: BTreeMap<String, Item>,
}

fn state_path(slug: &str) -> std::path::PathBuf {
    calendars::dir_of(slug).join("caldav.json")
}

pub fn load(slug: &str) -> DavState {
    let v: Value = fs::read_to_string(state_path(slug)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    let mut st = DavState { ctag: v["ctag"].as_str().unwrap_or("").to_string(), ..DavState::default() };
    for (href, i) in v["items"].as_object().into_iter().flatten() {
        st.items.insert(
            href.clone(),
            Item {
                etag: i["etag"].as_str().unwrap_or("").to_string(),
                hash: i["hash"].as_str().unwrap_or("").to_string(),
                ids: i["ids"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
                single: i["single"].as_bool().unwrap_or(false),
            },
        );
    }
    st
}

fn save(slug: &str, st: &DavState) {
    let items: serde_json::Map<String, Value> = st.items.iter().map(|(h, i)| (h.clone(), json!({"etag": i.etag, "hash": i.hash, "ids": i.ids, "single": i.single}))).collect();
    let _ = fs::write(state_path(slug), json!({"ctag": st.ctag, "items": items}).to_string());
}

/// Cannot this event be written back (its resource has several VEVENTs)?
pub fn locked(st: &DavState, id: &str) -> bool {
    st.items.values().any(|i| !i.single && i.ids.iter().any(|x| x == id))
}

fn plain(t: &str) -> String {
    t.lines().filter(|l| !l.starts_with("DTSTAMP:")).collect::<Vec<_>>().join("\n")
}

fn digest(t: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    plain(t).hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Files of the folder: id → text
fn local_files(slug: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for f in fs::read_dir(calendars::dir_of(slug)).into_iter().flatten().flatten() {
        let p = f.path();
        if p.extension().is_some_and(|e| e == "ics")
            && let (Some(id), Ok(text)) = (p.file_stem().and_then(|s| s.to_str()), fs::read_to_string(&p))
        {
            out.insert(id.to_string(), text);
        }
    }
    out
}

/// Does the folder differ from what was last synced (something to push)?
pub fn dirty(slug: &str) -> bool {
    let st = load(slug);
    let local = local_files(slug);
    let known = |id: &String| st.items.values().any(|i| i.ids.contains(id));
    local.keys().any(|id| !known(id)) || st.items.values().filter(|i| i.single).any(|i| local.get(&i.ids[0]).is_none_or(|t| digest(t) != i.hash))
}

// ───── sync ─────

#[derive(Default, Debug, PartialEq)]
pub struct Stats {
    pub pushed: usize,
    pub pulled: usize,
    pub removed: usize,
}

fn quoted(etag: &str) -> String {
    if etag.starts_with('"') || etag.starts_with("W/") { etag.to_string() } else { format!("\"{etag}\"") }
}

/// Writes the events of a resource as files of the folder; returns their ids
fn write_resource(slug: &str, text: &str) -> Vec<(String, String)> {
    let dir = calendars::dir_of(slug);
    let mut out = Vec::new();
    for e in ics::parse_ics(text) {
        let id = cli::short_hash(&format!("{slug}/{}", e.uid));
        let body = ics::to_ics(&e);
        if fs::write(dir.join(format!("{id}.ics")), &body).is_ok() {
            out.push((id, body));
        }
    }
    out
}

pub fn sync(t: &dyn Transport, slug: &str, collection: &str) -> Result<Stats, String> {
    let coll = with_slash(collection);
    let dir = calendars::dir_of(slug);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut st = load(slug);
    let mut stats = Stats::default();
    let local = local_files(slug);

    // 1. push what changed here
    for (href, item) in st.items.clone() {
        if !item.single {
            continue;
        }
        let id = &item.ids[0];
        match local.get(id) {
            None => {
                let r = t.request("DELETE", &href, &[("If-Match", &quoted(&item.etag))], None)?;
                if r.ok() || r.status == 404 {
                    st.items.remove(&href);
                    stats.pushed += 1;
                }
            }
            Some(text) if digest(text) != item.hash => {
                let r = t.request("PUT", &href, &[("Content-Type", "text/calendar; charset=utf-8"), ("If-Match", &quoted(&item.etag))], Some(text))?;
                if r.ok() {
                    let it = st.items.get_mut(&href).ok_or("lost item")?;
                    (it.etag, it.hash) = (r.header("etag"), digest(text));
                    stats.pushed += 1;
                } else if r.status == 412 {
                    // both sides changed: the server wins, the local version stays beside it
                    let _ = fs::write(dir.join(format!("{id}.conflict")), text);
                    st.items.get_mut(&href).ok_or("lost item")?.etag = String::new();
                    st.items.get_mut(&href).ok_or("lost item")?.hash = String::new();
                } else {
                    return Err(crate::i18n::trf("the server answered %1", &[&r.status]));
                }
            }
            _ => {}
        }
    }
    let known: Vec<String> = st.items.values().flat_map(|i| i.ids.clone()).collect();
    for (id, text) in &local {
        if known.contains(id) {
            continue;
        }
        let href = format!("{coll}{id}.ics");
        let r = t.request("PUT", &href, &[("Content-Type", "text/calendar; charset=utf-8"), ("If-None-Match", "*")], Some(text))?;
        if r.ok() {
            st.items.insert(href, Item { etag: r.header("etag"), hash: digest(text), ids: vec![id.clone()], single: true });
            stats.pushed += 1;
        } else if r.status != 412 {
            return Err(crate::i18n::trf("the server answered %1", &[&r.status]));
        }
    }

    // 2. pull what changed there
    let list = propfind(t, &coll, "1", "<d:getetag/><d:getcontenttype/><cs:getctag/>")?;
    let mut remote: BTreeMap<String, String> = BTreeMap::new();
    let mut ctag = String::new();
    for r in elements_of(&list.body) {
        let Some(href) = xml::first(r, "href").map(xml::text) else { continue };
        let url = join(&coll, &href);
        if same_resource(&url, &coll) {
            ctag = xml::first(r, "getctag").map(xml::text).unwrap_or_default();
        } else if let Some(etag) = xml::first(r, "getetag").map(xml::text).filter(|e| !e.is_empty()) {
            remote.insert(url, etag);
        }
    }
    let changed: Vec<String> = remote.iter().filter(|(h, e)| st.items.get(*h).is_none_or(|i| &i.etag != *e)).map(|(h, _)| h.clone()).collect();
    for batch in changed.chunks(25) {
        let hrefs: String = batch.iter().map(|h| format!("<d:href>{}</d:href>", xml::escape(&path_of(h)))).collect();
        let body = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<c:calendar-multiget xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\"><d:prop><d:getetag/><c:calendar-data/></d:prop>{hrefs}</c:calendar-multiget>");
        let r = t.request("REPORT", &coll, &[XML, ("Depth", "1")], Some(&body))?;
        if r.status != 207 {
            return Err(crate::i18n::trf("the server answered %1", &[&r.status]));
        }
        for resp in elements_of(&r.body) {
            let (Some(href), Some(data)) = (xml::first(resp, "href").map(xml::text), xml::first(resp, "calendar-data").map(xml::text)) else { continue };
            let url = join(&coll, &href);
            let etag = xml::first(resp, "getetag").map(xml::text).unwrap_or_default();
            let written = write_resource(slug, &data);
            let old = st.items.get(&url).map(|i| i.ids.clone()).unwrap_or_default();
            for gone in old.iter().filter(|o| !written.iter().any(|(id, _)| id == *o)) {
                let _ = fs::remove_file(dir.join(format!("{gone}.ics")));
            }
            let single = written.len() == 1;
            st.items.insert(url, Item { etag, hash: if single { digest(&written[0].1) } else { String::new() }, ids: written.iter().map(|(id, _)| id.clone()).collect(), single });
            stats.pulled += 1;
        }
    }
    // gone from the server
    for href in st.items.keys().filter(|h| !remote.contains_key(*h)).cloned().collect::<Vec<_>>() {
        if let Some(item) = st.items.remove(&href) {
            for id in &item.ids {
                let _ = fs::remove_file(dir.join(format!("{id}.ics")));
            }
            stats.removed += 1;
        }
    }
    st.ctag = ctag;
    save(slug, &st);
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A server in memory: the calendar at /cal/ with its resources
    #[derive(Default)]
    struct Fake {
        files: RefCell<BTreeMap<String, (String, String)>>,
        n: RefCell<u32>,
        log: RefCell<Vec<String>>,
    }
    impl Fake {
        fn put(&self, path: &str, text: &str) -> String {
            *self.n.borrow_mut() += 1;
            let etag = format!("\"e{}\"", self.n.borrow());
            self.files.borrow_mut().insert(path.to_string(), (etag.clone(), text.to_string()));
            etag
        }
    }
    impl Transport for Fake {
        fn request(&self, method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>) -> Result<Resp, String> {
            let path = path_of(url);
            let h = |k: &str| headers.iter().find(|(a, _)| a.eq_ignore_ascii_case(k)).map(|(_, v)| *v);
            self.log.borrow_mut().push(format!("{method} {path}"));
            let reply = |status: u16, headers: &str, body: String| Ok(Resp { status, headers: headers.to_string(), body });
            match method {
                "PROPFIND" if path == "/cal/" => {
                    let mut out = String::from("<d:multistatus xmlns:d=\"DAV:\" xmlns:cs=\"http://calendarserver.org/ns/\"><d:response><d:href>/cal/</d:href><d:propstat><d:prop><cs:getctag>ct</cs:getctag></d:prop></d:propstat></d:response>");
                    for (p, (e, _)) in self.files.borrow().iter() {
                        out += &format!("<d:response><d:href>{p}</d:href><d:propstat><d:prop><d:getetag>{e}</d:getetag></d:prop></d:propstat></d:response>");
                    }
                    reply(207, "", out + "</d:multistatus>")
                }
                "REPORT" => {
                    let mut out = String::from("<d:multistatus xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\">");
                    for href in xml::elements(body.unwrap_or(""), "href") {
                        if let Some((e, t)) = self.files.borrow().get(&xml::text(href)) {
                            out += &format!("<d:response><d:href>{href}</d:href><d:propstat><d:prop><d:getetag>{e}</d:getetag><c:calendar-data>{}</c:calendar-data></d:prop></d:propstat></d:response>", xml::escape(t));
                        }
                    }
                    reply(207, "", out + "</d:multistatus>")
                }
                "PUT" => {
                    let exists = self.files.borrow().get(&path).map(|(e, _)| e.clone());
                    if h("If-None-Match") == Some("*") && exists.is_some() || h("If-Match").is_some_and(|m| exists.as_deref() != Some(m)) {
                        return reply(412, "", String::new());
                    }
                    let etag = self.put(&path, body.unwrap_or(""));
                    reply(201, &format!("ETag: {etag}\n"), String::new())
                }
                "DELETE" => {
                    let exists = self.files.borrow().get(&path).map(|(e, _)| e.clone());
                    if h("If-Match").is_some_and(|m| exists.as_deref() != Some(m)) {
                        return reply(412, "", String::new());
                    }
                    self.files.borrow_mut().remove(&path);
                    reply(204, "", String::new())
                }
                _ => reply(404, "", String::new()),
            }
        }
    }

    fn ev(uid: &str, title: &str) -> String {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nDTSTART:20261201T090000\r\nSUMMARY:{title}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n")
    }

    #[test]
    fn a_calendar_is_pulled_changed_pushed_and_deleted_on_both_sides() {
        let _turn = cli::STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-dav-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the tests that set the store take turns (STORE_LOCK)
        unsafe { std::env::set_var("XDG_DATA_HOME", &tmp) };
        let c = calendars::create_account("Work", "https://dav.example.org/cal/", "me").unwrap();
        let server = Fake::default();
        server.put("/cal/a.ics", &ev("a@x", "From the server"));
        // a first sync pulls
        let s = sync(&server, &c.slug, &c.caldav.clone().unwrap()).unwrap();
        assert_eq!(s, Stats { pushed: 0, pulled: 1, removed: 0 });
        let id = cli::short_hash(&format!("{}/a@x", c.slug));
        assert!(fs::read_to_string(calendars::dir_of(&c.slug).join(format!("{id}.ics"))).unwrap().contains("From the server"));
        assert!(!dirty(&c.slug));
        // nothing changed: nothing happens
        assert_eq!(sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap(), Stats::default());
        // an event made here goes up, a changed one too
        let e = ics::Event { uid: "n@x".into(), summary: "Made here".into(), start: chrono::NaiveDate::from_ymd_opt(2026, 12, 2).unwrap().and_hms_opt(10, 0, 0).unwrap(), ..Default::default() };
        fs::write(calendars::dir_of(&c.slug).join("zzzz.ics"), ics::to_ics(&e)).unwrap();
        assert!(dirty(&c.slug));
        assert_eq!(sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap().pushed, 1);
        assert!(server.files.borrow().get("/cal/zzzz.ics").unwrap().1.contains("Made here"));
        let mut changed = ics::parse_ics(&fs::read_to_string(calendars::dir_of(&c.slug).join(format!("{id}.ics"))).unwrap()).remove(0);
        changed.summary = "Changed here".into();
        fs::write(calendars::dir_of(&c.slug).join(format!("{id}.ics")), ics::to_ics(&changed)).unwrap();
        assert_eq!(sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap().pushed, 1);
        assert!(server.files.borrow().get("/cal/a.ics").unwrap().1.contains("Changed here"));
        // changed on both sides: the server wins and the local version is kept
        let mut both = changed.clone();
        both.summary = "Local".into();
        fs::write(calendars::dir_of(&c.slug).join(format!("{id}.ics")), ics::to_ics(&both)).unwrap();
        server.put("/cal/a.ics", &ev("a@x", "Remote"));
        sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap();
        assert!(fs::read_to_string(calendars::dir_of(&c.slug).join(format!("{id}.ics"))).unwrap().contains("Remote"));
        assert!(fs::read_to_string(calendars::dir_of(&c.slug).join(format!("{id}.conflict"))).unwrap().contains("Local"));
        // deleted here, deleted there
        fs::remove_file(calendars::dir_of(&c.slug).join("zzzz.ics")).unwrap();
        server.files.borrow_mut().remove("/cal/a.ics");
        let s = sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap();
        assert!(server.files.borrow().is_empty() && s.pushed == 1 && s.removed == 1);
        assert!(!calendars::dir_of(&c.slug).join(format!("{id}.ics")).exists());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn a_resource_with_changed_occurrences_is_read_only() {
        let _turn = cli::STORE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!("fili-cal-dav2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        // SAFETY: the tests that set the store take turns (STORE_LOCK)
        unsafe { std::env::set_var("XDG_DATA_HOME", &tmp) };
        let c = calendars::create_account("Team", "https://dav.example.org/cal/", "me").unwrap();
        let server = Fake::default();
        let series = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:m\r\nDTSTART:20261123T100000\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\nSUMMARY:Class\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:m\r\nRECURRENCE-ID:20261130T100000\r\nDTSTART:20261130T140000\r\nSUMMARY:Moved\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        server.put("/cal/m.ics", series);
        sync(&server, &c.slug, "https://dav.example.org/cal/").unwrap();
        let st = load(&c.slug);
        let ids = &st.items.values().next().unwrap().ids;
        assert_eq!(ids.len(), 2);
        assert!(locked(&st, &ids[0]) && locked(&st, &ids[1]) && !locked(&st, "other"));
        // a locked resource is never pushed
        assert!(!dirty(&c.slug));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn urls_are_joined_and_discovery_reads_a_nextcloud_answer() {
        assert_eq!(join("https://h.example.org/dav/", "/cal/x/"), "https://h.example.org/cal/x/");
        assert_eq!(join("https://h.example.org/dav/", "y.ics"), "https://h.example.org/dav/y.ics");
        assert_eq!(path_of("https://h.example.org"), "/");
        struct Nc;
        impl Transport for Nc {
            fn request(&self, _m: &str, url: &str, _h: &[(&str, &str)], _b: Option<&str>) -> Result<Resp, String> {
                let body = match path_of(url).as_str() {
                    "/" => "<d:multistatus xmlns:d=\"DAV:\"><d:response><d:href>/</d:href><d:propstat><d:prop><d:current-user-principal><d:href>/principals/me/</d:href></d:current-user-principal></d:prop></d:propstat></d:response></d:multistatus>",
                    "/principals/me/" => "<d:multistatus xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\"><d:response><d:href>/principals/me/</d:href><d:propstat><d:prop><c:calendar-home-set><d:href>/calendars/me/</d:href></c:calendar-home-set></d:prop></d:propstat></d:response></d:multistatus>",
                    "/calendars/me/" => "<d:multistatus xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\"><d:response><d:href>/calendars/me/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat></d:response><d:response><d:href>/calendars/me/personal/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>Personal</d:displayname><c:supported-calendar-component-set><c:comp name=\"VEVENT\"/></c:supported-calendar-component-set></d:prop></d:propstat></d:response><d:response><d:href>/calendars/me/tasks/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>Tasks</d:displayname><c:supported-calendar-component-set><c:comp name=\"VTODO\"/></c:supported-calendar-component-set></d:prop></d:propstat></d:response></d:multistatus>",
                    _ => return Ok(Resp { status: 404, headers: String::new(), body: String::new() }),
                };
                Ok(Resp { status: 207, headers: String::new(), body: body.into() })
            }
        }
        let found = discover(&Nc, "https://cloud.example.org/").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].url.as_str(), found[0].name.as_str()), ("https://cloud.example.org/calendars/me/personal/", "Personal"));
    }
}
