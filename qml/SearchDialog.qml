// Search: a field and, as you type, the events of all time whose title, place or notes hold the text (the coming ones first). Enter
// or a click goes to the day of the event; arrows move in the list. `show()` opens it; `picked(date)` is for the owner to apply.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    required property var tools
    property var backend: null
    property var results: []
    property int index: 0
    signal picked(date day, string id)

    title: Fili.tr("Search")
    showButtons: false
    cardWidth: 520

    function show(text) {
        field.text = text || "";
        results = [];
        index = 0;
        run();
        open();
        field.focusField();
    }
    function run() {
        results = field.text.trim() === "" ? [] : JSON.parse(backend.search(field.text));
        index = 0;
    }
    function choose(i) {
        if (i < 0 || i >= results.length)
            return;
        const r = results[i];
        close();
        picked(tools.day(tools.parse(r.start)), r.id);
    }
    Keys.onDownPressed: index = Math.min(results.length - 1, index + 1)
    Keys.onUpPressed: index = Math.max(0, index - 1)

    FiliTextField {
        id: field
        width: parent.width
        placeholder: Fili.tr("Title, place or notes")
        onTextChanged: dlg.run()
        onAccepted: dlg.choose(dlg.index)
        Keys.onDownPressed: dlg.index = Math.min(dlg.results.length - 1, dlg.index + 1)
        Keys.onUpPressed: dlg.index = Math.max(0, dlg.index - 1)
    }
    Item {
        width: parent.width
        height: Math.min(300, Math.max(36, dlg.results.length * 40))
        ListView {
            id: list
            anchors.fill: parent
            clip: true
            model: dlg.results
            currentIndex: dlg.index
            boundsBehavior: Flickable.StopAtBounds
            delegate: Item {
                id: row
                required property var modelData
                required property int index
                width: list.width
                height: 40
                Rectangle {
                    anchors { fill: parent; topMargin: 1; bottomMargin: 1 }
                    radius: FiliTheme.radiusControl
                    color: row.index === dlg.index || area.containsMouse ? FiliTheme.surfaceHover : "transparent"
                }
                Rectangle { x: FiliTheme.space2; y: 8; width: 3; height: 24; radius: 1.5; color: dlg.tools.colorOf({ color: row.modelData.color }) }
                Column {
                    anchors { left: parent.left; leftMargin: FiliTheme.space2 + 12; right: parent.right; rightMargin: FiliTheme.space2; verticalCenter: parent.verticalCenter }
                    Text { width: parent.width; text: row.modelData.title; elide: Text.ElideRight; font.family: FiliTheme.font; font.pixelSize: FiliTheme.size; color: FiliTheme.text }
                    Text {
                        width: parent.width
                        text: Qt.locale().toString(dlg.tools.parse(row.modelData.start), row.modelData.allDay ? "ddd d MMM yyyy" : "ddd d MMM yyyy, HH:mm") + (row.modelData.place ? " · " + row.modelData.place : "") + " · " + row.modelData.calendar
                        elide: Text.ElideRight
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.sizeSmall
                        color: FiliTheme.muted
                    }
                }
                MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onClicked: dlg.choose(row.index) }
                Accessible.role: Accessible.ListItem
                Accessible.name: row.modelData.title
            }
        }
        Text {
            visible: dlg.results.length === 0
            anchors.centerIn: parent
            text: field.text.trim() === "" ? "" : Fili.tr("No event found")
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.size
            color: FiliTheme.muted
        }
    }
}
