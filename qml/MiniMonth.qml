// Small month of the sidebar: a click on a day goes there, today is a disc, the selected day a tint, a dot marks a day with events.
import QtQuick
import Fili.Agenda
import Fili.UI

Item {
    id: m
    required property var tools
    property date anchor: new Date()
    property date today: new Date()
    property var events: []
    property int firstDay: 1
    // The month shown, apart from the day selected (the arrows of the mini calendar)
    property int shift: 0
    readonly property date month: new Date(anchor.getFullYear(), anchor.getMonth() + shift, 1)
    readonly property date gridStart: tools.weekStart(month, firstDay)
    readonly property real cell: width / 7
    signal picked(date d)
    implicitHeight: 28 + 18 + 6 * cell
    onAnchorChanged: shift = 0

    Item {
        width: parent.width
        height: 28
        Text {
            anchors { left: parent.left; leftMargin: FiliTheme.space2; verticalCenter: parent.verticalCenter }
            text: Qt.locale().monthName(m.month.getMonth(), Locale.LongFormat) + " " + m.month.getFullYear()
            font.family: FiliTheme.font
            font.pixelSize: FiliTheme.size
            font.weight: FiliTheme.weightAccent
            color: FiliTheme.text
        }
        FiliIconButton { anchors { right: nextBtn.left; verticalCenter: parent.verticalCenter }
            name: "chevron-left"; iconSize: 14; width: 22; tip: Fili.tr("Previous month"); onClicked: m.shift -= 1 }
        FiliIconButton { id: nextBtn; anchors { right: parent.right; rightMargin: 2; verticalCenter: parent.verticalCenter }
            name: "chevron-right"; iconSize: 14; width: 22; tip: Fili.tr("Next month"); onClicked: m.shift += 1 }
    }
    Row {
        y: 28
        Repeater {
            model: 7
            Text {
                required property int index
                width: m.cell
                height: 18
                horizontalAlignment: Text.AlignHCenter
                text: Qt.locale().dayName((m.firstDay + index) % 7, Locale.NarrowFormat).toUpperCase()
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeLabel
                color: FiliTheme.muted
            }
        }
    }
    Repeater {
        model: 42
        Item {
            id: d
            required property int index
            readonly property date date: m.tools.addDays(m.gridStart, index)
            readonly property bool inMonth: date.getMonth() === m.month.getMonth()
            readonly property bool isToday: m.tools.sameDay(date, m.today)
            readonly property bool selected: m.tools.sameDay(date, m.anchor)
            readonly property bool busy: m.events.some(ev => m.tools.onDay(ev, date))
            x: (index % 7) * m.cell
            y: 46 + Math.floor(index / 7) * m.cell
            width: m.cell
            height: m.cell
            Rectangle {
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height) - 4
                height: width
                radius: width / 2
                color: d.isToday ? FiliTheme.accentColor : d.selected ? FiliTheme.surfaceHover : dayArea.containsMouse ? FiliTheme.alpha(FiliTheme.surfaceHover, 0.6) : "transparent"
            }
            Text {
                anchors.centerIn: parent
                text: d.date.getDate()
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeList
                font.weight: d.isToday || d.selected ? FiliTheme.weightAccent : Font.Normal
                color: d.isToday ? (0.299 * FiliTheme.accentColor.r + 0.587 * FiliTheme.accentColor.g + 0.114 * FiliTheme.accentColor.b > 0.55 ? "#14181f" : "#ffffff") : d.inMonth ? FiliTheme.text : FiliTheme.muted
            }
            Rectangle {
                visible: d.busy && !d.isToday
                anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 2 }
                width: 4
                height: 4
                radius: 2
                color: FiliTheme.accentColor
            }
            MouseArea { id: dayArea; anchors.fill: parent; hoverEnabled: true; onClicked: m.picked(d.date) }
            Accessible.role: Accessible.Button
            Accessible.name: Qt.formatDate(d.date, "d MMMM yyyy")
        }
    }
}
