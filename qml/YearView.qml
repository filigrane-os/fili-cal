// Year: twelve small months, a day with events tinted by how busy it is; a click on a day goes to its month.
import QtQuick
import Fili.Agenda
import Fili.UI

Item {
    id: v
    required property var tools
    property date anchor: new Date()
    property var events: []
    property date today: new Date()
    property int firstDay: 1
    signal dayPicked(date d)

    readonly property int year: anchor.getFullYear()
    // Number of events by day (YYYY-MM-DD)
    readonly property var busy: {
        const m = {};
        for (const e of events) {
            const s = tools.day(tools.start(e));
            const last = e.allDay && e.end ? tools.addDays(tools.day(tools.parse(e.end)), -1) : tools.day(tools.end(e));
            for (let d = s; d <= last && d <= tools.addDays(s, 40); d = tools.addDays(d, 1))
                m[tools.iso(d)] = (m[tools.iso(d)] || 0) + 1;
        }
        return m;
    }
    readonly property real monthW: (width - 2 * FiliTheme.margin - 2 * gap) / 3
    readonly property real gap: FiliTheme.space5
    readonly property real cell: Math.max(14, Math.min(monthW / 7, 30))

    Flickable {
        id: flick
        anchors { fill: parent; rightMargin: scroll.width }
        clip: true
        contentWidth: width
        contentHeight: grid.height + 2 * FiliTheme.margin
        boundsBehavior: Flickable.StopAtBounds
        KineticScroll {}
        Grid {
            id: grid
            x: FiliTheme.margin
            y: FiliTheme.margin
            columns: 3
            columnSpacing: v.gap
            rowSpacing: v.gap
            Repeater {
                model: 12
                Item {
                    id: mo
                    required property int index
                    readonly property date first: new Date(v.year, index, 1)
                    readonly property date gridStart: v.tools.weekStart(first, v.firstDay)
                    width: v.monthW
                    height: 22 + 16 + 6 * v.cell
                    Text {
                        text: Qt.locale().monthName(mo.index, Locale.LongFormat)
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.size
                        font.weight: FiliTheme.weightAccent
                        color: v.today.getFullYear() === v.year && v.today.getMonth() === mo.index ? FiliTheme.accentColor : FiliTheme.text
                        MouseArea { anchors.fill: parent; onClicked: v.dayPicked(mo.first) }
                    }
                    Row {
                        y: 22
                        Repeater {
                            model: 7
                            Text {
                                required property int index
                                width: v.cell
                                height: 16
                                horizontalAlignment: Text.AlignHCenter
                                text: Qt.locale().dayName((v.firstDay + index) % 7, Locale.NarrowFormat).toUpperCase()
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
                            readonly property date date: v.tools.addDays(mo.gridStart, index)
                            readonly property bool inMonth: date.getMonth() === mo.index
                            readonly property int count: inMonth ? (v.busy[v.tools.iso(date)] || 0) : 0
                            readonly property bool isToday: v.tools.sameDay(date, v.today)
                            visible: inMonth
                            x: (index % 7) * v.cell
                            y: 38 + Math.floor(index / 7) * v.cell
                            width: v.cell
                            height: v.cell
                            Rectangle {
                                anchors { fill: parent; margins: 1 }
                                radius: FiliTheme.radiusControl
                                // a busier day is more tinted
                                color: d.isToday ? FiliTheme.accentColor : d.count > 0 ? FiliTheme.alpha(FiliTheme.accentColor, Math.min(0.55, 0.12 + 0.1 * d.count)) : areaD.containsMouse ? FiliTheme.surfaceHover : "transparent"
                            }
                            Text {
                                anchors.centerIn: parent
                                text: d.date.getDate()
                                font.family: FiliTheme.font
                                font.pixelSize: FiliTheme.sizeSmall
                                color: d.isToday ? v.tools.inkOn(FiliTheme.accentColor) : FiliTheme.text
                            }
                            MouseArea { id: areaD; anchors.fill: parent; hoverEnabled: true; onClicked: v.dayPicked(d.date) }
                            Accessible.role: Accessible.Button
                            Accessible.name: Qt.locale().toString(d.date, "d MMMM yyyy") + (d.count > 0 ? ", " + d.count : "")
                        }
                    }
                }
            }
        }
    }
    FiliScrollBar { id: scroll; flick: flick; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
}
