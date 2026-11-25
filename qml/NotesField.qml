// A field of several lines for the notes of an event: it wraps, grows to a fixed height and scrolls, Enter makes a new line (the
// dialog is saved with its button). Looks like FiliTextField (the same border, the accent on focus).
import QtQuick
import Fili.UI

Rectangle {
    id: f
    property alias text: edit.text
    property string placeholder
    readonly property bool hasFocus: edit.activeFocus
    function focusField() { edit.forceActiveFocus(); }

    implicitWidth: 180
    implicitHeight: 96
    radius: FiliTheme.radiusControl
    color: FiliTheme.surface
    border.width: FiliTheme.borderWidth
    border.color: edit.activeFocus ? FiliTheme.accentColor : FiliTheme.border
    Accessible.role: Accessible.EditableText
    Accessible.name: placeholder

    Flickable {
        id: flick
        anchors { fill: parent; margins: FiliTheme.space2 }
        clip: true
        contentWidth: width
        contentHeight: edit.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        // The cursor stays in sight while typing
        function follow() {
            const r = edit.cursorRectangle;
            if (r.y < contentY)
                contentY = r.y;
            else if (r.y + r.height > contentY + height)
                contentY = r.y + r.height - height;
        }
        TextEdit {
            id: edit
            width: flick.width
            wrapMode: TextEdit.Wrap
            selectByMouse: true
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.sizeList
            color: FiliTheme.text
            selectionColor: FiliTheme.accentColor
            selectedTextColor: FiliTheme.background
            activeFocusOnTab: true
            onCursorRectangleChanged: flick.follow()
            Text {
                visible: edit.text === "" && !edit.preeditText
                text: f.placeholder
                font: edit.font
                color: FiliTheme.muted
            }
        }
    }
}
