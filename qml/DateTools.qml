// Date helpers shared by the views (one instance in Main.qml, handed to the others as `tools`). Dates are local; the events
// of the store carry "YYYY-MM-DDTHH:MM" (a timed event) or "YYYY-MM-DD" (a whole day).
import QtQuick
import Fili.UI

QtObject {
    // Accent tones of the theme by name ([dark, dark bright, light, light bright]), and whether the theme is light: the colour of
    // a calendar is the name of one of them
    property var accents: ({})
    property bool light: false
    function colorOf(ev) {
        const a = ev && ev.color ? accents[ev.color] : undefined;
        return a ? (light ? a[2] : a[0]) : FiliTheme.accentColor;
    }
    // Text on a colour: dark on a light one, white on a dark one
    function inkOn(c) { return 0.299 * c.r + 0.587 * c.g + 0.114 * c.b > 0.55 ? "#14181f" : "#ffffff"; }
    // The settings of fili-cal.toml (Main.qml sets them): clock12, firstHour, defaultLength (minutes), defaultAlert (seconds)
    property var prefs: ({ firstDay: -1, clock12: false, firstHour: 7, defaultLength: 60, defaultAlert: 600 })
    // "14:30" or "2:30 PM", as the settings say
    function hhmm(h, m) {
        if (!prefs.clock12)
            return pad(h) + ":" + pad(m);
        return (h % 12 === 0 ? 12 : h % 12) + (m === 0 ? "" : ":" + pad(m)) + (h < 12 ? " AM" : " PM");
    }
    // ISO number of the week of a date
    function isoWeek(d) {
        const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
        t.setUTCDate(t.getUTCDate() + 4 - (t.getUTCDay() || 7));
        return Math.ceil(((t - Date.UTC(t.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
    }
    function pad(n) { return (n < 10 ? "0" : "") + n; }
    function iso(d) { return d.getFullYear() + "-" + pad(d.getMonth() + 1) + "-" + pad(d.getDate()); }
    function stamp(d) { return iso(d) + "T" + pad(d.getHours()) + ":" + pad(d.getMinutes()); }
    function parse(s) {
        const m = /^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?/.exec(s || "");
        return m ? new Date(+m[1], +m[2] - 1, +m[3], m[4] ? +m[4] : 0, m[5] ? +m[5] : 0) : new Date(NaN);
    }
    function day(d) { return new Date(d.getFullYear(), d.getMonth(), d.getDate()); }
    function addDays(d, n) { return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, d.getHours(), d.getMinutes()); }
    function addMonths(d, n) {
        const last = new Date(d.getFullYear(), d.getMonth() + n + 1, 0).getDate();
        return new Date(d.getFullYear(), d.getMonth() + n, Math.min(d.getDate(), last));
    }
    // First day of the week of the language (Qt: 0 Sunday … 6 Saturday)
    function weekStart(d, first) { return addDays(day(d), -((d.getDay() - first + 7) % 7)); }
    function sameDay(a, b) { return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate(); }
    function start(ev) { return parse(ev.start); }
    function end(ev) { return ev.end ? parse(ev.end) : parse(ev.start); }
    // The occurrence touches the day (a whole day is on its day only)
    function onDay(ev, d) {
        const s = start(ev), e = end(ev), a = day(d), b = addDays(a, 1);
        // a whole day lasts from its first day to the day before its end (DTEND is the day after the last one)
        if (ev.allDay)
            return a >= day(s) && a < (ev.end && day(e) > day(s) ? day(e) : addDays(day(s), 1));
        return s < b && (e > a || s >= a);
    }
    function eventsOn(events, d) { return events.filter(ev => onDay(ev, d)); }
    function clock(ev) { const s = start(ev); return hhmm(s.getHours(), s.getMinutes()); }
    // A reminder: an alarm at the start and no end
    function isReminder(ev) { return !ev.allDay && !ev.end && ev.alarms.length > 0 && ev.alarms[0] === 0; }
    // Events of a day side by side without covering each other: [{ev, s, e, col, cols}] (minutes of the day)
    function layout(events, d) {
        const a = day(d), list = [];
        for (const ev of events) {
            if (ev.allDay || !onDay(ev, d))
                continue;
            const s = Math.max(0, Math.round((start(ev) - a) / 60000));
            let e = Math.min(1440, Math.round((end(ev) - a) / 60000));
            e = Math.max(e, s + 30);
            list.push({ ev: ev, s: s, e: e, col: 0, cols: 1 });
        }
        list.sort((x, y) => x.s - y.s || y.e - x.e);
        const out = [];
        let cols = [], cluster = [], clusterEnd = -1;
        const flush = () => { cluster.forEach(c => c.cols = cols.length); out.push(...cluster); cluster = []; cols = []; clusterEnd = -1; };
        for (const it of list) {
            if (cluster.length > 0 && it.s >= clusterEnd)
                flush();
            let c = 0;
            while (c < cols.length && cols[c] > it.s)
                c++;
            cols[c] = it.e;
            it.col = c;
            cluster.push(it);
            clusterEnd = Math.max(clusterEnd, it.e);
        }
        if (cluster.length > 0)
            flush();
        return out;
    }
}
