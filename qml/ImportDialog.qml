// Import an iCalendar file or link: its path or link (a file dropped on the window, or handed over by the system, fills it), a
// preview of what it holds, the calendar to add it to. `show(source)` opens it; `imported(source, calendar)` is for the owner to apply.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    // The Agenda object (cal) that previews and imports
    property var backend: null
    property var calendars: []
    property string calendar: "personal"
    property var info: ({ ok: false })
    signal imported(string source, string calendar)

    title: Fili.tr("Import events")
    confirmLabel: Fili.tr("Add to calendar")
    cardWidth: 460
    confirmEnabled: info.ok === true

    function show(source) {
        sourceField.text = source || "";
        calendar = (calendars.find(c => !c.hidden && !c.subscribed) || calendars.find(c => !c.subscribed) || { slug: "personal" }).slug;
        refresh();
        open();
        sourceField.focusField();
    }
    function refresh() {
        const s = sourceField.text.trim();
        info = s === "" ? { ok: false } : JSON.parse(backend.previewImport(s));
    }
    onAccepted: imported(sourceField.text.trim(), calendar)

    Text {
        width: parent.width
        wrapMode: Text.WordWrap
        text: Fili.tr("Drop an .ics file here, or type its path or link.")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    FiliTextField { id: sourceField; width: parent.width; placeholder: Fili.tr("File or link"); onTextChanged: dlg.refresh(); onAccepted: dlg.accept() }
    Text {
        visible: !dlg.info.ok && dlg.info.error !== undefined && sourceField.text.trim() !== ""
        width: parent.width
        wrapMode: Text.WordWrap
        text: dlg.info.error || ""
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.danger
    }
    Text {
        visible: dlg.info.ok === true
        text: (dlg.info.count === 1 ? Fili.tr("1 event") : Fili.tr("%1 events")).replace("%1", dlg.info.count)
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.size
        font.weight: FiliTheme.weightAccent
        color: FiliTheme.text
    }
    Column {
        visible: dlg.info.ok === true
        width: parent.width
        Repeater {
            model: dlg.info.ok ? dlg.info.events.slice(0, 5) : []
            Text {
                required property var modelData
                width: parent.width
                elide: Text.ElideRight
                text: modelData.start.replace("T", " ") + "  " + modelData.title
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: FiliTheme.textSecondary
            }
        }
    }
    FiliSelect {
        visible: dlg.info.ok === true
        width: parent.width
        model: dlg.calendars.filter(c => !c.subscribed).map(c => ({ label: c.name, value: c.slug }))
        current: dlg.calendar
        onPicked: v => dlg.calendar = v
    }
}
