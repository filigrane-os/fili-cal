// Editor of an event, in a dialog: title, whole day, date and times, repeat, alert, place, notes. `edit(ev)` opens it on an
// occurrence (a series asks nothing: the change is for the whole series), `create(date)` on a new event. Enter saves, Esc
// closes. `saved(id, json)` (id empty for a new event) and `deleted(id, occ, all)` are for the owner to apply.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: ed
    required property var tools
    property string eventId: ""
    property string occ: ""
    // Start of the first occurrence of a series (the series moves by as many days as the date of this occurrence)
    property string first: ""
    property bool series: false
    // The event comes from a subscription: it can be read, not changed
    property bool readOnly: false
    // Files attached to the event (paths): a file dropped on the card is added; the owner of the file is Casier, not Agenda
    property var attachments: []
    property bool done: false
    // The meeting link of the event (found in the place or the notes), and its service
    property string join: ""
    property string joinService: ""
    // Every meeting link of the event: [{url, service}] (a button each on the card)
    property var joins: []
    // All the meeting links of the event (taken out of the notes shown)
    property var joinUrls: []
    signal joinRequested(string url)
    // The calendars [{slug, name, color, hidden}] and the one the event is in
    property var calendars: []
    property string calendar: "personal"
    // The occurrence and the first start as wall times of the zone of the event (the editor works in that zone)
    property string occZ: ""
    // The rule has parts the editor cannot show (BYDAY, BYSETPOS…): it is kept as it is and shown as text
    property bool custom: false
    property string rrule: ""
    // The alerts of the event, in seconds before it (0 = at the time, negative = after): as many as the user wants
    property var alertList: []
    readonly property bool isNew: eventId === ""
    // The event is shown first, not editable; the Edit button switches to the form (a new event opens on the form)
    property bool editing: false
    readonly property bool viewing: !isNew && !editing
    // Duration kept when the start moves: the end is "start + length" until the user types one
    property bool allDay: false

    // The card has the title and the cross of the kit; its buttons are Join (one per service), then Edit when the event can change
    title: isNew ? Fili.tr("New event") : Fili.tr("Event")
    showCancel: !viewing
    showConfirm: !viewing
    actions: [
        Repeater {
            model: ed.viewing ? ed.joins : []
            FiliButton { required property var modelData; label: Fili.tr("Join %1").replace("%1", modelData.service); primary: true; onClicked: ed.joinRequested(modelData.url) }
        },
        FiliButton {
            visible: ed.viewing && !ed.readOnly
            label: Fili.tr("Edit")
            primary: ed.joins.length === 0
            onClicked: { ed.editing = true; titleField.focusField(); }
        }
    ]
    confirmLabel: Fili.tr("Save")
    cardWidth: 460
    confirmEnabled: !readOnly && titleField.text.trim() !== "" && startOk && endOk && lastOk && zoneOk
    readonly property bool dateOk: /^\d{4}-\d{2}-\d{2}$/.test(dateField.text) && !isNaN(tools.parse(dateField.text).getTime())
    readonly property bool timeOk: /^([01]\d|2[0-3]):[0-5]\d$/.test(startField.text)
    readonly property bool startOk: dateOk && (allDay || timeOk)
    readonly property bool endOk: allDay || endField.text === "" || (/^([01]\d|2[0-3]):[0-5]\d$/.test(endField.text) && endField.text >= startField.text)
    // A whole-day event can last several days: the last day, on or after the first
    readonly property bool lastOk: !allDay || lastField.text === "" || (/^\d{4}-\d{2}-\d{2}$/.test(lastField.text) && !isNaN(tools.parse(lastField.text).getTime()) && lastField.text >= dateField.text)
    readonly property bool zoneOk: /^(|UTC|[A-Za-z][A-Za-z_+\-]*(\/[A-Za-z0-9_+\-]+)*)$/.test(zoneField.text.trim())
    signal saved(string id, string json)
    signal deleted(string id, string occ, bool all)

    readonly property var repeats: [
        { label: Fili.tr("Never"), value: "" }, { label: Fili.tr("Every day"), value: "day" }, { label: Fili.tr("Every week"), value: "week" },
        { label: Fili.tr("Every month"), value: "month" }, { label: Fili.tr("Every year"), value: "year" }]
    // The usual choices; an alert of the file that is none of them is listed too
    readonly property var presets: [0, 300, 600, 900, 1800, 3600, 7200, 86400, 172800, 604800]
    function alertLabel(v) {
        if (v === 0)
            return Fili.tr("At the time of the event");
        const n = Math.abs(v), after = v < 0;
        const [count, unit] = n % 604800 === 0 ? [n / 604800, "w"] : n % 86400 === 0 ? [n / 86400, "d"] : n % 3600 === 0 ? [n / 3600, "h"] : [Math.round(n / 60), "m"];
        const many = after ? { m: Fili.tr("%1 minutes after"), h: Fili.tr("%1 hours after"), d: Fili.tr("%1 days after"), w: Fili.tr("%1 weeks after") }
            : { m: Fili.tr("%1 minutes before"), h: Fili.tr("%1 hours before"), d: Fili.tr("%1 days before"), w: Fili.tr("%1 weeks before") };
        const one = { m: Fili.tr("1 minute before"), h: Fili.tr("1 hour before"), d: Fili.tr("1 day before"), w: Fili.tr("1 week before") };
        return count === 1 && !after ? one[unit] : many[unit].replace("%1", count);
    }
    function alertChoices(current) {
        const values = presets.includes(current) ? presets : presets.concat([current]).sort((x, y) => x - y);
        return values.map(v => ({ label: alertLabel(v), value: v }));
    }
    function setAlert(i, v) { const l = alertList.slice(); l[i] = v; alertList = l; }
    function removeAlert(i) { const l = alertList.slice(); l.splice(i, 1); alertList = l; }
    // Adds the first usual choice that the event does not have yet (10 minutes before when it has none)
    function addAlert() { alertList = alertList.concat([[600, 0, 300, 900, 1800, 3600, 7200, 86400].find(v => !alertList.includes(v)) ?? 600]); }
    property string every: ""

    // `directly`: open on the form (the Edit entry of the context menu) instead of on the view
    function edit(ev, directly) {
        editing = directly === true;
        eventId = ev.id;
        occ = ev.occ;
        calendar = ev.calendar;
        readOnly = ev.readonly === true;
        attachments = ev.attachments || [];
        done = ev.done === true;
        join = ev.join || "";
        joinService = ev.joinService || "";
        joins = ev.joins || [];
        joinUrls = ev.joinUrls || [];
        first = ev.zfirst;
        occZ = ev.zstart;
        series = ev.series;
        custom = ev.custom;
        rrule = ev.rrule;
        zoneField.text = ev.tz;
        const s = tools.parse(ev.zstart);
        allDay = ev.allDay;
        titleField.text = ev.title;
        dateField.text = tools.iso(s);
        startField.text = ev.allDay ? "09:00" : tools.pad(s.getHours()) + ":" + tools.pad(s.getMinutes());
        const e = ev.zend ? tools.parse(ev.zend) : null;
        endField.text = e && !ev.allDay ? tools.pad(e.getHours()) + ":" + tools.pad(e.getMinutes()) : "";
        // the end of a whole-day event is the day after its last one
        lastField.text = e && ev.allDay ? tools.iso(tools.addDays(e, -1)) : tools.iso(s);
        placeField.text = ev.place;
        noteField.text = ev.note;
        every = ev.every;
        alertList = ev.alarms.slice().sort((x, y) => y - x);
        open();
        if (editing)
            titleField.focusField();
    }
    // `date` is the start (midnight = a whole day); `until` the end when it was dragged out
    function create(date, until) {
        editing = true;
        eventId = "";
        occ = "";
        first = "";
        series = false;
        occZ = "";
        readOnly = false;
        attachments = [];
        done = false;
        join = "";
        joinService = "";
        joins = [];
        joinUrls = [];
        calendar = (calendars.find(c => !c.hidden && !c.subscribed) || { slug: "personal" }).slug;
        custom = false;
        rrule = "";
        zoneField.text = "";
        allDay = !until && date.getHours() === 0 && date.getMinutes() === 0;
        titleField.text = "";
        dateField.text = tools.iso(date);
        lastField.text = tools.iso(date);
        const h = allDay ? 9 : date.getHours(), m = allDay ? 0 : date.getMinutes();
        startField.text = tools.pad(h) + ":" + tools.pad(m);
        const len = tools.prefs.defaultLength, eh = Math.min(23, h + Math.floor((m + len) / 60));
        if (until)
            endField.text = tools.pad(until.getHours()) + ":" + tools.pad(until.getMinutes());
        else
            endField.text = tools.pad(eh) + ":" + tools.pad(eh === h + Math.floor((m + len) / 60) ? (m + len) % 60 : 59);
        placeField.text = "";
        noteField.text = "";
        every = "";
        alertList = allDay || tools.prefs.defaultAlert < 0 ? [] : [tools.prefs.defaultAlert];
        open();
        titleField.focusField();
    }
    // A file dropped on the window while the editor is open: attached to this event
    function attach(url) {
        const path = decodeURIComponent(String(url).replace(/^file:\/\//, ""));
        if (path.startsWith("/") && !attachments.includes(path))
            attachments = attachments.concat([path]);
    }
    // Whole days dragged over in the month: a whole-day event from `first` to `last`
    function createDays(first, last) {
        create(first);
        allDay = true;
        lastField.text = tools.iso(last);
    }
    onAccepted: {
        // A series keeps its own first day: it moves by the days this occurrence was moved
        let day = dateField.text;
        if (series && first !== "") {
            const delta = Math.round((tools.parse(dateField.text) - tools.day(tools.parse(occZ))) / 86400000);
            day = tools.iso(tools.addDays(tools.parse(first), delta));
        }
        // the days a whole-day event lasts, counted from its first day; DTEND is the day after the last one
        const span = allDay && lastField.text > dateField.text ? Math.round((tools.parse(lastField.text) - tools.parse(dateField.text)) / 86400000) : 0;
        const o = {
            title: titleField.text.trim(), allDay: allDay,
            start: allDay ? day : day + "T" + startField.text,
            end: allDay ? (span > 0 ? tools.iso(tools.addDays(tools.parse(day), span + 1)) : "") : (endField.text === "" ? "" : day + "T" + endField.text),
            tz: zoneField.text.trim(), calendar: calendar, attachments: attachments, done: done,
            place: placeField.text, note: noteField.text, every: every, alarms: alertList
        };
        saved(eventId, JSON.stringify(o));
    }

    component Row2: Row {
        property alias label: lab.text
        default property alias field: slot.data
        width: parent ? parent.width : 0
        spacing: FiliTheme.space3
        Text {
            id: lab
            width: 84
            height: 26
            verticalAlignment: Text.AlignVCenter
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.size
            color: FiliTheme.muted
        }
        Row { id: slot; spacing: FiliTheme.space2 }
    }

    Text {
        visible: ed.readOnly && !ed.viewing
        width: parent.width
        wrapMode: Text.WordWrap
        text: Fili.tr("This event comes from a subscription: it cannot be changed here.")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    // ---- The event, to read: what it is, when, where, the meeting to join; Edit switches to the form ----
    readonly property string whenLine: {
        const L = Qt.locale(), d = tools.parse(dateField.text);
        if (isNaN(d.getTime()))
            return "";
        const day = L.toString(d, "dddd d MMMM yyyy");
        if (allDay) {
            const last = tools.parse(lastField.text);
            return !isNaN(last.getTime()) && last > d ? day + " → " + L.toString(last, "dddd d MMMM yyyy") : day;
        }
        const hm = t => { const m = /^(\d\d):(\d\d)$/.exec(t); return m ? tools.hhmm(+m[1], +m[2]) : t; };
        return day + ", " + hm(startField.text) + (endField.text !== "" ? " – " + hm(endField.text) : "") + (zoneField.text.trim() !== "" ? " (" + zoneField.text.trim() + ")" : "");
    }
    readonly property string repeatLine: custom ? Fili.tr("Custom rule") : (repeats.find(r => r.value === every && every !== "") || { label: "" }).label
    readonly property string alertLine: alertList.map(v => alertLabel(v)).join(", ")
    // The notes without the meeting link (it is the button above)
    readonly property string notesShown: {
        // the meeting links are the buttons above: a line that only said "Join:" before its link goes with it
        const lines = noteField.text.split("\n").map(l => {
            let t = l;
            for (const u of joinUrls)
                t = t.split("<" + u + ">").join("").split(u).join("");
            return t !== l && (t.trim() === "" || /^[^:]{0,24}:\s*$/.test(t.trim())) ? null : t;
        });
        // the long rules of an invitation (_____) are not shown; empty lines come once
        return lines.filter(l => l !== null && !/^\s*[_=*-]{6,}\s*$/.test(l)).join("\n").replace(/\n{3,}/g, "\n\n").trim();
    }
    // A text as the card shows it: the links can be clicked, the rest is text (a link is shortened on screen, not in what it opens)
    function linked(text) {
        // a link between < > is a link: the brackets of a mail are not shown
        text = text.replace(/(\S)<(https?:\/\/[^\s<>"']+)>/g, "$1 $2").replace(/<(https?:\/\/[^\s<>"']+)>/g, "$1");
        const esc = t => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\r?\n/g, "<br>");
        const re = /https?:\/\/[^\s<>"']+/g;
        let out = "", last = 0, m;
        while ((m = re.exec(text)) !== null) {
            const url = m[0].replace(/[.,;:!?)\]]+$/, "");
            out += esc(text.slice(last, m.index)) + '<a href="' + url.replace(/&/g, "&amp;").replace(/"/g, "&quot;") + '">' + esc(url.length > 64 ? url.slice(0, 61) + "…" : url) + "</a>";
            last = m.index + url.length;
            re.lastIndex = last;
        }
        return out + esc(text.slice(last));
    }
    // Only web links are opened from the text of an event
    function openLink(link) { if (/^https?:\/\//.test(link)) Fili.openPath(link); }
    readonly property string calendarName: (calendars.find(c => c.slug === calendar) || { name: "" }).name
    Column {
        visible: ed.viewing
        width: parent.width
        spacing: FiliTheme.space2
        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            text: titleField.text
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.sizeTitle
            font.weight: FiliTheme.weightAccent
            font.strikeout: ed.done
            color: FiliTheme.text
        }
        Text {
            width: parent.width
            wrapMode: Text.WordWrap
            text: ed.whenLine
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.size
            color: FiliTheme.text
        }
        Text {
            visible: ed.repeatLine !== "" || ed.alertLine !== ""
            width: parent.width
            wrapMode: Text.WordWrap
            text: [ed.repeatLine, ed.alertLine !== "" ? Fili.tr("Alert") + ": " + ed.alertLine : ""].filter(t => t !== "").join(" · ")
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.sizeSmall
            color: FiliTheme.muted
        }
        Text {
            visible: placeField.text !== "" && !ed.joins.some(j => j.url === placeField.text.trim())
            width: parent.width
            wrapMode: Text.WordWrap
            textFormat: Text.StyledText
            linkColor: FiliTheme.accentColor
            text: "📍 " + ed.linked(placeField.text)
            onLinkActivated: link => ed.openLink(link)
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.size
            color: FiliTheme.text
            HoverHandler { cursorShape: parent.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor }
        }
        // The notes can be long (the text of a Teams invitation): a window of their own
        Flickable {
            visible: ed.notesShown !== ""
            width: parent.width
            height: Math.min(noteText.implicitHeight, 230)
            contentHeight: noteText.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            Text {
                id: noteText
                width: parent.width
                wrapMode: Text.Wrap
                textFormat: Text.StyledText
                linkColor: FiliTheme.accentColor
                text: ed.linked(ed.notesShown)
                onLinkActivated: link => ed.openLink(link)
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                color: FiliTheme.textSecondary
                HoverHandler { cursorShape: parent.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor }
            }
        }
        Row {
            visible: ed.calendarName !== ""
            spacing: FiliTheme.space2
            Rectangle { anchors.verticalCenter: parent.verticalCenter; width: 10; height: 10; radius: 3; color: ed.tools.colorOf({ color: (ed.calendars.find(c => c.slug === ed.calendar) || { color: "default" }).color }) }
            Text { text: ed.calendarName; font.family: FiliTheme.font; font.pixelSize: FiliTheme.sizeSmall; color: FiliTheme.muted }
        }
        Repeater {
            model: ed.viewing ? ed.attachments : []
            Text {
                required property string modelData
                width: parent.width
                elide: Text.ElideMiddle
                text: "📎 " + modelData.split("/").pop()
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                color: viewAtt.containsMouse ? FiliTheme.accentColor : FiliTheme.text
                MouseArea { id: viewAtt; anchors.fill: parent; hoverEnabled: true; onClicked: Fili.openPath(modelData) }
            }
        }
        Text {
            visible: ed.readOnly
            width: parent.width
            wrapMode: Text.WordWrap
            text: Fili.tr("This event comes from a subscription: it cannot be changed here.")
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.sizeSmall
            color: FiliTheme.muted
        }
    }
    FiliTextField { visible: !ed.viewing; id: titleField; width: parent.width; placeholder: Fili.tr("Title"); onAccepted: ed.accept() }
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("All day")
        FiliSwitch { anchors.verticalCenter: parent.verticalCenter; checked: ed.allDay; onToggled: ed.allDay = !ed.allDay }
    }
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("Date")
        FiliTextField { id: dateField; width: 120; placeholder: "2026-12-24"; error: !ed.dateOk && text !== ""; onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.viewing && (ed.allDay)
        label: Fili.tr("Last day")
        FiliTextField { id: lastField; width: 120; placeholder: "2026-12-26"; error: !ed.lastOk; onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.viewing && (!ed.allDay)
        label: Fili.tr("Time")
        FiliTextField { id: startField; width: 70; placeholder: "14:30"; error: !ed.timeOk && text !== ""; onAccepted: ed.accept() }
        Text { height: 26; verticalAlignment: Text.AlignVCenter; text: "→"; font.family: FiliTheme.font; font.pixelSize: FiliTheme.size; color: FiliTheme.muted }
        FiliTextField { id: endField; width: 70; placeholder: "15:30"; error: !ed.endOk; onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.viewing && (!ed.allDay)
        label: Fili.tr("Time zone")
        FiliTextField { id: zoneField; width: 200; placeholder: Fili.tr("Same hour everywhere"); error: !ed.zoneOk; onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.viewing && (ed.calendars.length > 1)
        label: Fili.tr("Calendar")
        FiliSelect { width: 200; model: ed.calendars.filter(c => !c.subscribed || c.slug === ed.calendar).map(c => ({ label: c.name, value: c.slug })); current: ed.calendar; onPicked: v => ed.calendar = v }
    }
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("Repeat")
        FiliSelect { width: 200; model: ed.repeats; current: ed.every; onPicked: v => ed.every = v }
    }
    Text {
        visible: !ed.viewing && (ed.custom && ed.every !== "")
        width: parent.width
        elide: Text.ElideRight
        text: Fili.tr("Custom rule, kept as it is: %1").replace("%1", ed.rrule)
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    // One line for each alert, a button to add another
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("Alerts")
        Column {
            spacing: FiliTheme.space1
            Repeater {
                model: ed.alertList
                Row {
                    id: alertRow
                    required property int modelData
                    required property int index
                    spacing: FiliTheme.space1
                    FiliSelect { width: 200; model: ed.alertChoices(alertRow.modelData); current: alertRow.modelData; onPicked: v => ed.setAlert(alertRow.index, v) }
                    FiliIconButton { anchors.verticalCenter: parent.verticalCenter; name: "x"; iconSize: 12; tip: Fili.tr("Remove this alert"); onClicked: ed.removeAlert(alertRow.index) }
                }
            }
            Text {
                visible: ed.alertList.length === 0
                height: 26
                verticalAlignment: Text.AlignVCenter
                text: Fili.tr("None")
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                color: FiliTheme.muted
            }
            FiliButton { label: Fili.tr("Add an alert"); iconName: "plus"; onClicked: ed.addAlert() }
        }
    }
    Row2 {
        visible: !ed.viewing && (ed.allDay && !ed.isNew)
        label: Fili.tr("Status")
        FiliCheckBox { checked: ed.done; label: Fili.tr("Done"); onToggled: ed.done = !ed.done }
    }
    Row2 {
        visible: !ed.viewing && (ed.join !== "")
        label: Fili.tr("Meeting")
        JoinButton { tools: ed.tools; url: ed.join; service: ed.joinService; labelled: true; onClicked: ed.joinRequested(ed.join) }
    }
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("Place")
        FiliTextField { id: placeField; width: ed.cardWidth - 2 * FiliTheme.space5 - 84 - FiliTheme.space3; placeholder: Fili.tr("Where"); onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.viewing
        label: Fili.tr("Notes")
        NotesField { id: noteField; width: ed.cardWidth - 2 * FiliTheme.space5 - 84 - FiliTheme.space3; height: 110; placeholder: Fili.tr("Notes") }
    }
    // Attached files: drop one on this card; a click opens it with the application of the user
    Column {
        visible: !ed.viewing
        width: parent.width
        spacing: 2
        Text {
            visible: ed.attachments.length === 0 && !ed.readOnly
            text: Fili.tr("Drop a file here to attach it")
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.sizeSmall
            color: FiliTheme.muted
        }
        Repeater {
            model: ed.attachments
            Item {
                id: att
                required property string modelData
                width: parent.width
                height: 22
                Text {
                    anchors { left: parent.left; right: rm.left; rightMargin: FiliTheme.space2; verticalCenter: parent.verticalCenter }
                    text: "📎 " + att.modelData.split("/").pop()
                    elide: Text.ElideMiddle
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.size
                    color: attArea.containsMouse ? FiliTheme.accentColor : FiliTheme.text
                    MouseArea { id: attArea; anchors.fill: parent; hoverEnabled: true; onClicked: Fili.openPath(att.modelData) }
                }
                FiliIconButton {
                    id: rm
                    anchors.right: parent.right
                    visible: !ed.readOnly
                    name: "x"
                    iconSize: 12
                    tip: Fili.tr("Remove the attachment")
                    onClicked: ed.attachments = ed.attachments.filter(p => p !== att.modelData)
                }
                Accessible.role: Accessible.Button
                Accessible.name: att.modelData
            }
        }
    }
    Row {
        visible: !ed.viewing && (!ed.isNew && !ed.readOnly)
        spacing: FiliTheme.space2
        FiliButton { label: ed.series ? Fili.tr("Delete this occurrence") : Fili.tr("Delete"); onClicked: { ed.close(); ed.deleted(ed.eventId, ed.occ, false); } }
        FiliButton { visible: ed.series; label: Fili.tr("Delete the series"); onClicked: { ed.close(); ed.deleted(ed.eventId, ed.occ, true); } }
    }
}
