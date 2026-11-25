//! Just enough XML to read the answers of a CalDAV server (WebDAV `multistatus`): elements are found by their local name, the
//! prefix (`d:`, `D:`, `ns0:`, none) is ignored, and the text of an element is unescaped. Not a general parser: no
//! validation, no DTD, comments and processing instructions are skipped.

/// The start tag at `xml[at..]` (which begins with '<'): (local name, prefixed name, index after '>', self-closing)
fn start_tag(xml: &str, at: usize) -> Option<(&str, &str, usize, bool)> {
    let rest = &xml[at + 1..];
    if rest.starts_with(['/', '?', '!']) {
        return None;
    }
    let end = rest.find('>')?;
    let tag = &rest[..end];
    let name_end = tag.find(|c: char| c.is_whitespace() || c == '/').unwrap_or(tag.len());
    let name = &tag[..name_end];
    let local = name.rsplit(':').next().unwrap_or(name);
    Some((local, name, at + 1 + end + 1, tag.ends_with('/')))
}

/// The inner text (raw, not unescaped) of every element with this local name, in order
pub fn elements<'a>(xml: &'a str, local: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = xml[from..].find('<') {
        let at = from + i;
        // CDATA and comments are skipped as a whole
        if xml[at..].starts_with("<![CDATA[") {
            from = xml[at..].find("]]>").map_or(xml.len(), |e| at + e + 3);
            continue;
        }
        if xml[at..].starts_with("<!--") {
            from = xml[at..].find("-->").map_or(xml.len(), |e| at + e + 3);
            continue;
        }
        let Some((name, prefixed, after, empty)) = start_tag(xml, at) else {
            from = at + 1;
            continue;
        };
        if name != local {
            from = after;
            continue;
        }
        if empty {
            out.push("");
            from = after;
            continue;
        }
        // the closing tag with the same prefix; elements of one name are not nested in the answers we read
        let close = format!("</{prefixed}>");
        match xml[after..].find(&close) {
            Some(e) => {
                out.push(&xml[after..after + e]);
                from = after + e + close.len();
            }
            None => break,
        }
    }
    out
}

pub fn first<'a>(xml: &'a str, local: &str) -> Option<&'a str> {
    elements(xml, local).into_iter().next()
}

/// Is there an element of this local name (empty or not), e.g. `<c:calendar/>` in a `resourcetype`
pub fn has(xml: &str, local: &str) -> bool {
    let mut from = 0;
    while let Some(i) = xml[from..].find('<') {
        let at = from + i;
        if let Some((name, _, after, _)) = start_tag(xml, at) {
            if name == local {
                return true;
            }
            from = after;
        } else {
            from = at + 1;
        }
    }
    false
}

/// The value of an attribute of the first element of this name (`<c:comp name="VEVENT"/>`)
pub fn attrs_of<'a>(xml: &'a str, local: &str, attr: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = xml[from..].find('<') {
        let at = from + i;
        if let Some((name, _, after, _)) = start_tag(xml, at) {
            if name == local {
                let tag = &xml[at..after];
                if let Some(p) = tag.find(&format!("{attr}=\"")) {
                    let v = &tag[p + attr.len() + 2..];
                    if let Some(e) = v.find('"') {
                        out.push(&v[..e]);
                    }
                }
            }
            from = after;
        } else {
            from = at + 1;
        }
    }
    out
}

/// Text of an element: CDATA taken as it is, entities unescaped, trimmed
pub fn text(inner: &str) -> String {
    let mut out = String::new();
    let mut rest = inner;
    while let Some(i) = rest.find("<![CDATA[") {
        out.push_str(&unescape(&rest[..i]));
        let body = &rest[i + 9..];
        match body.find("]]>") {
            Some(e) => {
                out.push_str(&body[..e]);
                rest = &body[e + 3..];
            }
            None => {
                out.push_str(body);
                rest = "";
            }
        }
    }
    out.push_str(&unescape(rest));
    out.trim().to_string()
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#13;", "\r").replace("&amp;", "&")
}

/// Escapes text for an element or an attribute
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
 <D:response><D:href>/cal/home/</D:href>
  <D:propstat><D:prop><D:resourcetype><D:collection/><C:calendar/></D:resourcetype><D:displayname>Work &amp; Co</D:displayname>
   <C:supported-calendar-component-set><C:comp name="VEVENT"/><C:comp name="VTODO"/></C:supported-calendar-component-set></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
 <d:response><d:href>/cal/home/a.ics</d:href><d:propstat><d:prop><d:getetag>"abc"</d:getetag><c:calendar-data><![CDATA[BEGIN:VCALENDAR
END:VCALENDAR]]></c:calendar-data></d:prop></d:propstat></d:response>
</D:multistatus>"#;

    #[test]
    fn elements_are_found_whatever_the_prefix() {
        let r = elements(ANSWER, "response");
        assert_eq!(r.len(), 2);
        assert_eq!(text(first(r[0], "href").unwrap()), "/cal/home/");
        assert_eq!(text(first(r[0], "displayname").unwrap()), "Work & Co");
        assert!(has(first(r[0], "resourcetype").unwrap(), "calendar"));
        assert_eq!(attrs_of(r[0], "comp", "name"), ["VEVENT", "VTODO"]);
        assert_eq!(text(first(r[1], "getetag").unwrap()), "\"abc\"");
        assert!(text(first(r[1], "calendar-data").unwrap()).starts_with("BEGIN:VCALENDAR\nEND"));
        assert!(!has(r[1], "collection"));
    }
}
