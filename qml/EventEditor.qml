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
    readonly property bool isNew: eventId === ""
    // Duration kept when the start moves: the end is "start + length" until the user types one
    property bool allDay: false

    title: isNew ? Fili.tr("New event") : Fili.tr("Event")
    confirmLabel: Fili.tr("Save")
    cardWidth: 460
    confirmEnabled: titleField.text.trim() !== "" && startOk && endOk
    readonly property bool dateOk: /^\d{4}-\d{2}-\d{2}$/.test(dateField.text) && !isNaN(tools.parse(dateField.text).getTime())
    readonly property bool timeOk: /^([01]\d|2[0-3]):[0-5]\d$/.test(startField.text)
    readonly property bool startOk: dateOk && (allDay || timeOk)
    readonly property bool endOk: allDay || endField.text === "" || (/^([01]\d|2[0-3]):[0-5]\d$/.test(endField.text) && endField.text >= startField.text)
    signal saved(string id, string json)
    signal deleted(string id, string occ, bool all)

    readonly property var repeats: [
        { label: Fili.tr("Never"), value: "" }, { label: Fili.tr("Every day"), value: "day" }, { label: Fili.tr("Every week"), value: "week" },
        { label: Fili.tr("Every month"), value: "month" }, { label: Fili.tr("Every year"), value: "year" }]
    readonly property var alerts: [
        { label: Fili.tr("None"), value: -1 }, { label: Fili.tr("At the time of the event"), value: 0 }, { label: Fili.tr("5 minutes before"), value: 300 },
        { label: Fili.tr("10 minutes before"), value: 600 }, { label: Fili.tr("15 minutes before"), value: 900 },
        { label: Fili.tr("30 minutes before"), value: 1800 }, { label: Fili.tr("1 hour before"), value: 3600 }, { label: Fili.tr("1 day before"), value: 86400 }]
    property string every: ""
    property int alert: 600

    function edit(ev) {
        eventId = ev.id;
        occ = ev.occ;
        first = ev.first;
        series = ev.series;
        const s = tools.start(ev);
        allDay = ev.allDay;
        titleField.text = ev.title;
        dateField.text = tools.iso(s);
        startField.text = ev.allDay ? "09:00" : tools.pad(s.getHours()) + ":" + tools.pad(s.getMinutes());
        endField.text = ev.end && !ev.allDay ? tools.pad(tools.end(ev).getHours()) + ":" + tools.pad(tools.end(ev).getMinutes()) : "";
        placeField.text = ev.place;
        noteField.text = ev.note;
        every = ev.every;
        alert = ev.alarms.length > 0 ? ev.alarms[0] : -1;
        open();
        titleField.focusField();
    }
    function create(date) {
        eventId = "";
        occ = "";
        first = "";
        series = false;
        allDay = date.getHours() === 0 && date.getMinutes() === 0;
        titleField.text = "";
        dateField.text = tools.iso(date);
        const h = allDay ? 9 : date.getHours(), m = allDay ? 0 : date.getMinutes();
        startField.text = tools.pad(h) + ":" + tools.pad(m);
        endField.text = tools.pad(Math.min(23, h + 1)) + ":" + tools.pad(m);
        placeField.text = "";
        noteField.text = "";
        every = "";
        alert = allDay ? -1 : 600;
        open();
        titleField.focusField();
    }
    onAccepted: {
        // A series keeps its own first day: it moves by the days this occurrence was moved
        let day = dateField.text;
        if (series && first !== "") {
            const delta = Math.round((tools.parse(dateField.text) - tools.day(tools.parse(occ))) / 86400000);
            day = tools.iso(tools.addDays(tools.parse(first), delta));
        }
        const o = {
            title: titleField.text.trim(), allDay: allDay,
            start: allDay ? day : day + "T" + startField.text,
            end: allDay || endField.text === "" ? "" : day + "T" + endField.text,
            place: placeField.text, note: noteField.text, every: every, alarms: alert < 0 ? [] : [alert]
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

    FiliTextField { id: titleField; width: parent.width; placeholder: Fili.tr("Title"); onAccepted: ed.accept() }
    Row2 {
        label: Fili.tr("All day")
        FiliSwitch { anchors.verticalCenter: parent.verticalCenter; checked: ed.allDay; onToggled: ed.allDay = !ed.allDay }
    }
    Row2 {
        label: Fili.tr("Date")
        FiliTextField { id: dateField; width: 120; placeholder: "2026-12-24"; error: !ed.dateOk && text !== ""; onAccepted: ed.accept() }
    }
    Row2 {
        visible: !ed.allDay
        label: Fili.tr("Time")
        FiliTextField { id: startField; width: 70; placeholder: "14:30"; error: !ed.timeOk && text !== ""; onAccepted: ed.accept() }
        Text { height: 26; verticalAlignment: Text.AlignVCenter; text: "→"; font.family: FiliTheme.font; font.pixelSize: FiliTheme.size; color: FiliTheme.muted }
        FiliTextField { id: endField; width: 70; placeholder: "15:30"; error: !ed.endOk; onAccepted: ed.accept() }
    }
    Row2 {
        label: Fili.tr("Repeat")
        FiliSelect { width: 200; model: ed.repeats; current: ed.every; onPicked: v => ed.every = v }
    }
    Row2 {
        label: Fili.tr("Alert")
        FiliSelect { width: 200; model: ed.alerts; current: ed.alert; onPicked: v => ed.alert = v }
    }
    Row2 {
        label: Fili.tr("Place")
        FiliTextField { id: placeField; width: ed.cardWidth - 2 * FiliTheme.space5 - 84 - FiliTheme.space3; placeholder: Fili.tr("Where"); onAccepted: ed.accept() }
    }
    Row2 {
        label: Fili.tr("Notes")
        FiliTextField { id: noteField; width: ed.cardWidth - 2 * FiliTheme.space5 - 84 - FiliTheme.space3; placeholder: Fili.tr("Notes"); onAccepted: ed.accept() }
    }
    Row {
        visible: !ed.isNew
        spacing: FiliTheme.space2
        FiliButton { label: ed.series ? Fili.tr("Delete this occurrence") : Fili.tr("Delete"); onClicked: { ed.close(); ed.deleted(ed.eventId, ed.occ, false); } }
        FiliButton { visible: ed.series; label: Fili.tr("Delete the series"); onClicked: { ed.close(); ed.deleted(ed.eventId, ed.occ, true); } }
    }
}
