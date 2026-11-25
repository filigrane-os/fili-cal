#!/usr/bin/env python3
"""A small CalDAV server for trying `fili-cal calendar connect` without an account: one user (me / secret) with one calendar
(Work) at /cal/me/work/. Not a conformance tool: just PROPFIND, REPORT calendar-multiget, GET, PUT and DELETE with the ETag rules.

    python3 tests/fake-caldav.py [PORT]      (default 8766)
"""
import base64
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

FILES = {}  # path -> (etag, text)
N = [0]
HOME = "/cal/me/"
COLL = "/cal/me/work/"
DAV = 'xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/"'


def ms(*responses):
    return '<?xml version="1.0"?><d:multistatus %s>%s</d:multistatus>' % (DAV, "".join(responses))


def resp(href, prop):
    return "<d:response><d:href>%s</d:href><d:propstat><d:prop>%s</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>" % (href, prop)


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def authed(self):
        want = "Basic " + base64.b64encode(b"me:secret").decode()
        if self.headers.get("Authorization") != want:
            self.send_response(401)
            self.send_header("WWW-Authenticate", 'Basic realm="x"')
            self.send_header("Content-Length", "0")
            self.end_headers()
            return False
        return True

    def body(self):
        n = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(n).decode() if n else ""

    def send(self, code, text="", headers=()):
        data = text.encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/xml; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        for k, v in headers:
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(data)

    def do_PROPFIND(self):
        if not self.authed():
            return
        self.body()
        p = self.path
        if p in ("/", "/.well-known/caldav"):
            self.send(207, ms(resp("/", "<d:current-user-principal><d:href>/principals/me/</d:href></d:current-user-principal>")))
        elif p == "/principals/me/":
            self.send(207, ms(resp(p, "<c:calendar-home-set><d:href>%s</d:href></c:calendar-home-set>" % HOME)))
        elif p == HOME:
            self.send(207, ms(resp(HOME, "<d:resourcetype><d:collection/></d:resourcetype>"),
                              resp(COLL, '<d:resourcetype><d:collection/><c:calendar/></d:resourcetype><d:displayname>Work</d:displayname><c:supported-calendar-component-set><c:comp name="VEVENT"/></c:supported-calendar-component-set>')))
        elif p == COLL:
            rows = [resp(COLL, "<cs:getctag>%d</cs:getctag>" % N[0])]
            rows += [resp(k, "<d:getetag>%s</d:getetag>" % v[0]) for k, v in sorted(FILES.items())]
            self.send(207, ms(*rows))
        else:
            self.send(404)

    def do_REPORT(self):
        if not self.authed():
            return
        import re
        hrefs = re.findall(r"<d:href>([^<]*)</d:href>", self.body())
        rows = []
        for h in hrefs:
            if h in FILES:
                e, t = FILES[h]
                t = t.replace("&", "&amp;").replace("<", "&lt;")
                rows.append(resp(h, "<d:getetag>%s</d:getetag><c:calendar-data>%s</c:calendar-data>" % (e, t)))
        self.send(207, ms(*rows))

    def do_GET(self):
        if not self.authed():
            return
        if self.path in FILES:
            self.send(200, FILES[self.path][1], [("ETag", FILES[self.path][0])])
        else:
            self.send(404)

    def do_PUT(self):
        if not self.authed():
            return
        text, cur = self.body(), FILES.get(self.path)
        if self.headers.get("If-None-Match") == "*" and cur:
            return self.send(412)
        m = self.headers.get("If-Match")
        if m and (not cur or cur[0] != m):
            return self.send(412)
        N[0] += 1
        etag = '"e%d"' % N[0]
        FILES[self.path] = (etag, text)
        self.send(201, "", [("ETag", etag)])

    def do_DELETE(self):
        if not self.authed():
            return
        m, cur = self.headers.get("If-Match"), FILES.get(self.path)
        if m and (not cur or cur[0] != m):
            return self.send(412)
        FILES.pop(self.path, None)
        N[0] += 1
        self.send(204)


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8766
    FILES[COLL + "seed.ics"] = ('"e0"', "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:seed@x\r\nDTSTART:20261203T090000\r\nSUMMARY:From the server\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n")
    HTTPServer(("127.0.0.1", port), H).serve_forever()
