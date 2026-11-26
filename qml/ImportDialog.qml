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
    // An invitation was answered: "accept", "maybe" or "decline"
    signal answered(string source, string answer, string calendar)
    readonly property var invite: info.ok === true && info.invite ? info.invite : null
    // The file was handed over (by Facteur, the system, a drop): no field to type it in
    property bool given: false

    title: invite ? Fili.tr("Invitation") : Fili.tr("Import events")
    confirmLabel: invite ? Fili.tr("Add without answering") : Fili.tr("Add to calendar")
    cardWidth: 460
    confirmEnabled: info.ok === true

    function show(source) {
        given = !!source;
        sourceField.text = source || "";
        calendar = (calendars.find(c => !c.hidden && !c.subscribed) || calendars.find(c => !c.subscribed) || { slug: "personal" }).slug;
        refresh();
        open();
        if (!given)
            sourceField.focusField();
    }
    function refresh() {
        const s = sourceField.text.trim();
        info = s === "" ? { ok: false } : JSON.parse(backend.previewImport(s));
    }
    onAccepted: imported(sourceField.text.trim(), calendar)

    Text {
        visible: !dlg.given
        width: parent.width
        wrapMode: Text.WordWrap
        text: Fili.tr("Drop an .ics file here, or type its path or link.")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    FiliTextField { id: sourceField; visible: !dlg.given; width: parent.width; placeholder: Fili.tr("File or link"); onTextChanged: dlg.refresh(); onAccepted: dlg.accept() }
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
    // An invitation (from an e-mail): the three answers; the answer is sent to the organizer by Facteur
    Rectangle {
        visible: dlg.invite !== null
        width: parent.width
        height: inviteCol.implicitHeight + 2 * FiliTheme.space3
        radius: FiliTheme.radiusControl
        color: FiliTheme.surfaceHover
        Column {
            id: inviteCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: FiliTheme.space3 }
            spacing: FiliTheme.space2
            Text {
                width: parent.width
                text: dlg.invite ? Fili.tr("Invitation from %1").replace("%1", dlg.invite.name) : ""
                elide: Text.ElideRight
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                font.weight: FiliTheme.weightAccent
                color: FiliTheme.text
            }
            Row {
                spacing: FiliTheme.space2
                FiliButton { label: Fili.tr("Accept"); primary: true; onClicked: { dlg.close(); dlg.answered(sourceField.text.trim(), "accept", dlg.calendar); } }
                FiliButton { label: Fili.tr("Maybe"); onClicked: { dlg.close(); dlg.answered(sourceField.text.trim(), "maybe", dlg.calendar); } }
                FiliButton { label: Fili.tr("Decline"); onClicked: { dlg.close(); dlg.answered(sourceField.text.trim(), "decline", dlg.calendar); } }
            }
            Text {
                visible: dlg.invite && dlg.invite.account !== ""
                width: parent.width
                wrapMode: Text.WordWrap
                text: dlg.invite ? Fili.tr("Accept and Maybe add the event to the calendar, Decline does not; each answer is sent to %1 from %2.").replace("%1", dlg.invite.organizer).replace("%2", dlg.invite.account) : ""
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: FiliTheme.muted
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
