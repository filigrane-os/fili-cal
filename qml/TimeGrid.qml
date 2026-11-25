// Day and week: a header with the days, a strip for the whole-day events, then the hours (scrolling) with the events as blocks
// placed by time, side by side when they overlap, and a line at the current time. A double click on an hour opens a new event
// there, a click on a block opens it.
import QtQuick
import Fili.Agenda
import Fili.UI

Item {
    id: v
    required property var tools
    // First column and number of columns (1 day, 7 week)
    property date first: new Date()
    property int days: 7
    property var events: []
    property date today: new Date()
    property date now: new Date()
    readonly property real hourH: 52
    readonly property real gutter: 54
    readonly property real colW: (width - gutter - scroll.width) / days
    readonly property var columns: Array.from({ length: days }, (_, i) => tools.addDays(first, i))
    signal slotActivated(date at)
    signal eventClicked(var ev)

    function whole(d) { return v.tools.eventsOn(v.events, d).filter(e => e.allDay); }
    readonly property int wholeRows: Math.max(0, ...columns.map(d => whole(d).length))
    readonly property real headH: 44 + (wholeRows > 0 ? wholeRows * 20 + 6 : 0)

    // Header: day names and numbers (today in the accent colour), then the whole-day events
    Item {
        id: head
        width: parent.width
        height: v.headH
        Repeater {
            model: v.columns
            Item {
                id: hc
                required property var modelData
                required property int index
                readonly property bool isToday: v.tools.sameDay(modelData, v.today)
                x: v.gutter + index * v.colW
                width: v.colW
                height: v.headH
                Column {
                    x: FiliTheme.space2
                    y: 6
                    spacing: 0
                    Text {
                        text: Qt.locale().dayName(hc.modelData.getDay(), Locale.ShortFormat).toUpperCase()
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.sizeSmall
                        font.weight: FiliTheme.weightAccent
                        font.letterSpacing: 0.8
                        color: hc.isToday ? FiliTheme.accentColor : FiliTheme.muted
                    }
                    Text {
                        text: hc.modelData.getDate()
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.sizeTitle
                        font.weight: Font.Light
                        color: hc.isToday ? FiliTheme.accentColor : FiliTheme.text
                    }
                }
                Column {
                    x: 3
                    y: 44
                    width: parent.width - 6
                    spacing: 2
                    Repeater {
                        model: v.whole(hc.modelData)
                        Rectangle {
                            id: wc
                            required property var modelData
                            width: parent.width
                            height: 18
                            radius: 4
                            color: FiliTheme.accentColor
                            Text {
                                anchors { left: parent.left; leftMargin: 5; right: parent.right; rightMargin: 4; verticalCenter: parent.verticalCenter }
                                text: wc.modelData.title
                                elide: Text.ElideRight
                                font.family: FiliTheme.font
                                font.pixelSize: FiliTheme.sizeSmall
                                color: 0.299 * FiliTheme.accentColor.r + 0.587 * FiliTheme.accentColor.g + 0.114 * FiliTheme.accentColor.b > 0.55 ? "#14181f" : "#ffffff"
                            }
                            MouseArea { anchors.fill: parent; onClicked: v.eventClicked(wc.modelData) }
                        }
                    }
                }
            }
        }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: FiliTheme.borderWidth; color: FiliTheme.border }
    }

    Flickable {
        id: flick
        anchors { top: head.bottom; bottom: parent.bottom; left: parent.left; right: scroll.left }
        clip: true
        contentWidth: width
        contentHeight: 24 * v.hourH + 12
        boundsBehavior: Flickable.StopAtBounds
        KineticScroll {}
        // Opens around 7:00 (or a little before now when it is today)
        Component.onCompleted: contentY = Math.max(0, Math.min(contentHeight - height, (v.columns.some(d => v.tools.sameDay(d, v.today)) ? Math.max(0, v.now.getHours() - 2) : v.tools.prefs.firstHour) * v.hourH))

        // Hours: label in the gutter, a line across
        Repeater {
            model: 24
            Item {
                required property int index
                y: 6 + index * v.hourH
                width: flick.width
                height: v.hourH
                Text {
                    visible: index > 0
                    x: 0
                    width: v.gutter - 8
                    y: -height / 2
                    horizontalAlignment: Text.AlignRight
                    text: v.tools.hhmm(index, 0)
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeSmall
                    color: FiliTheme.muted
                }
                Rectangle { x: v.gutter; width: parent.width - v.gutter; height: FiliTheme.borderWidth; color: FiliTheme.alpha(FiliTheme.border, index === 0 ? 0 : 0.6) }
            }
        }
        // Columns
        Repeater {
            model: v.columns
            Item {
                id: col
                required property var modelData
                required property int index
                readonly property bool isToday: v.tools.sameDay(modelData, v.today)
                x: v.gutter + index * v.colW
                y: 6
                width: v.colW
                height: 24 * v.hourH
                Rectangle { height: parent.height; width: FiliTheme.borderWidth; color: FiliTheme.alpha(FiliTheme.border, 0.6) }
                Rectangle { anchors.fill: parent; color: col.isToday ? FiliTheme.alpha(FiliTheme.accentColor, 0.05) : "transparent" }
                MouseArea {
                    anchors.fill: parent
                    onDoubleClicked: m => {
                        const mins = Math.floor(m.y / v.hourH * 2) * 30;
                        const d = v.tools.day(col.modelData);
                        d.setMinutes(mins);
                        v.slotActivated(d);
                    }
                }
                Repeater {
                    model: v.tools.layout(v.events, col.modelData)
                    Rectangle {
                        id: blk
                        required property var modelData
                        readonly property bool reminder: v.tools.isReminder(modelData.ev)
                        x: 2 + (col.width - 4) * modelData.col / modelData.cols
                        width: (col.width - 4) / modelData.cols - 2
                        y: modelData.s / 60 * v.hourH + 1
                        height: reminder ? 20 : Math.max(20, (modelData.e - modelData.s) / 60 * v.hourH - 2)
                        radius: 5
                        color: FiliTheme.alpha(FiliTheme.accentColor, blkArea.containsMouse ? 0.3 : 0.18)
                        clip: true
                        Rectangle { width: 3; height: parent.height - 4; y: 2; x: 2; radius: 1.5; color: FiliTheme.accentColor }
                        Column {
                            anchors { left: parent.left; leftMargin: 9; right: parent.right; rightMargin: 4; top: parent.top; topMargin: 3 }
                            Text {
                                width: parent.width
                                text: (blk.reminder ? "● " : "") + blk.modelData.ev.title
                                elide: Text.ElideRight
                                font.family: FiliTheme.font
                                font.pixelSize: FiliTheme.sizeList
                                font.weight: FiliTheme.weightAccent
                                color: FiliTheme.text
                            }
                            Text {
                                visible: blk.height > 36
                                width: parent.width
                                text: v.tools.clock(blk.modelData.ev) + (blk.modelData.ev.place ? " · " + blk.modelData.ev.place : "")
                                elide: Text.ElideRight
                                font.family: FiliTheme.font
                                font.pixelSize: FiliTheme.sizeSmall
                                color: FiliTheme.textSecondary
                            }
                        }
                        MouseArea { id: blkArea; anchors.fill: parent; hoverEnabled: true; onClicked: v.eventClicked(blk.modelData.ev) }
                        Accessible.role: Accessible.Button
                        Accessible.name: blk.modelData.ev.title
                    }
                }
            }
        }
        // Current time
        Item {
            readonly property int idx: v.columns.findIndex(d => v.tools.sameDay(d, v.today))
            visible: idx >= 0
            x: v.gutter
            y: 6 + (v.now.getHours() * 60 + v.now.getMinutes()) / 60 * v.hourH
            width: flick.width - v.gutter
            height: 1
            Rectangle { x: -4; y: -4; width: 9; height: 9; radius: 4.5; color: FiliTheme.danger }
            Rectangle { y: 0; width: parent.width; height: 1; color: FiliTheme.danger }
        }
    }
    FiliScrollBar { id: scroll; flick: flick; anchors { top: head.bottom; bottom: parent.bottom; right: parent.right } }
}
