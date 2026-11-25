// Export the events of a calendar (or all) to an .ics file: the calendar and the file or folder to write. `start()` opens it;
// `exported(calendar, path)` is for the owner to apply.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    property var calendars: []
    property string calendar: "all"
    signal exported(string calendar, string path)

    title: Fili.tr("Export events")
    confirmLabel: Fili.tr("Export")
    cardWidth: 440
    confirmEnabled: pathField.text.trim() !== ""

    function start() {
        calendar = "all";
        pathField.text = "~/agenda.ics";
        open();
        pathField.focusField();
    }
    onAccepted: exported(calendar, pathField.text.trim())

    FiliSelect {
        width: parent.width
        model: [{ label: Fili.tr("All calendars"), value: "all" }].concat(dlg.calendars.map(c => ({ label: c.name, value: c.slug })))
        current: dlg.calendar
        onPicked: v => dlg.calendar = v
    }
    FiliTextField { id: pathField; width: parent.width; placeholder: Fili.tr("File or folder to write"); onAccepted: dlg.accept() }
}
