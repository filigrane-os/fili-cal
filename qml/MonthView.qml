// Month grid: six weeks, the day number top left (today in a filled disc), the events of the day as small bars under it,
// "+n more" when they do not fit. A click selects a day, a double click opens a new event on it, a click on a bar opens it.
import QtQuick
import Fili.Agenda
import Fili.UI

Item {
    id: v
    required property var tools
    property date anchor: new Date()
    property var events: []
    property int firstDay: 1
    property date today: new Date()
    readonly property date gridStart: tools.weekStart(new Date(anchor.getFullYear(), anchor.getMonth(), 1), firstDay)
    readonly property real headH: 28
    // Week numbers on the left and a week without its weekend are settings; `colList` holds the positions (0 = first day of the week) shown
    readonly property real wkW: tools.prefs.weekNumbers === true ? 30 : 0
    readonly property var colList: [0, 1, 2, 3, 4, 5, 6].filter(p => !(tools.prefs.workWeek === true && ((firstDay + p) % 7 === 0 || (firstDay + p) % 7 === 6)))
    readonly property real cellW: (width - wkW) / colList.length
    // The selected event (id|occurrence), shown with a ring; the day whose events are all listed in a popover (-1 = none)
    property string selectedKey: ""
    property int moreIndex: -1
    signal eventContext(var ev, real x, real y)
    signal joinRequested(var ev)
    signal slotContext(date at, real x, real y)
    readonly property real cellH: Math.max(40, (height - headH) / 6)
    signal dayClicked(date d)
    signal dayActivated(date d)
    signal eventClicked(var ev)
    // A bar dragged to another day (`dDays` days later or earlier, Alt copies); days dragged over empty cells make a whole-day event
    signal eventMoved(var ev, int dDays, bool copy)
    signal daysCreated(date first, date last)
    // The cell under a dragged bar, and the range of cells dragged over (indexes of the 42 cells, -1 = none)
    property int dropIndex: -1
    property int rangeFrom: -1
    property int rangeTo: -1
    function indexAt(x, y) {
        return Math.max(0, Math.min(5, Math.floor((y - headH) / cellH))) * 7 + colList[Math.max(0, Math.min(colList.length - 1, Math.floor((x - wkW) / cellW)))];
    }
    function reset() { dropIndex = -1; rangeFrom = -1; rangeTo = -1; }

    // Weekday names
    Row {
        x: v.wkW
        height: v.headH
        Repeater {
            model: v.colList
            Item {
                required property int modelData
                width: v.cellW
                height: v.headH
                Text {
                    anchors { left: parent.left; leftMargin: FiliTheme.space2 + 2; verticalCenter: parent.verticalCenter }
                    text: Qt.locale().dayName((v.firstDay + modelData) % 7, Locale.ShortFormat).toUpperCase()
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeSmall
                    font.weight: FiliTheme.weightAccent
                    font.letterSpacing: 0.8
                    color: FiliTheme.muted
                }
            }
        }
    }
    Rectangle { y: v.headH; width: parent.width; height: FiliTheme.borderWidth; color: FiliTheme.border }

    Repeater {
        model: 42
        Rectangle {
            id: cell
            required property int index
            readonly property date date: v.tools.addDays(v.gridStart, index)
            readonly property bool inMonth: date.getMonth() === v.anchor.getMonth()
            readonly property bool isToday: v.tools.sameDay(date, v.today)
            readonly property bool selected: v.tools.sameDay(date, v.anchor)
            readonly property var dayEvents: v.tools.eventsOn(v.events, date)
            readonly property int room: Math.max(0, Math.floor((height - 28) / 18))
            readonly property int shown: dayEvents.length > room ? Math.max(0, room - 1) : dayEvents.length
            readonly property int slot: v.colList.indexOf(index % 7)
            visible: slot >= 0
            Accessible.role: Accessible.ListItem
            Accessible.name: Qt.locale().toString(date, "dddd d MMMM yyyy") + (dayEvents.length > 0 ? ", " + dayEvents.length : "")
            x: v.wkW + slot * v.cellW
            y: v.headH + Math.floor(index / 7) * v.cellH
            width: v.cellW
            height: v.cellH
            readonly property bool targeted: v.dropIndex === index || (v.rangeFrom >= 0 && v.rangeTo >= 0 && index >= Math.min(v.rangeFrom, v.rangeTo) && index <= Math.max(v.rangeFrom, v.rangeTo))
            color: targeted ? FiliTheme.alpha(FiliTheme.accentColor, 0.2) : selected ? FiliTheme.alpha(FiliTheme.accentColor, 0.08) : "transparent"
            // Grid lines: right and bottom
            Rectangle { anchors.right: parent.right; height: parent.height; width: FiliTheme.borderWidth; color: FiliTheme.alpha(FiliTheme.border, 0.6) }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: FiliTheme.borderWidth; color: FiliTheme.alpha(FiliTheme.border, 0.6) }
            MouseArea {
                anchors.fill: parent
                onClicked: v.dayClicked(cell.date)
                onDoubleClicked: v.dayActivated(cell.date)
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onPressed: m => {
                    if (m.button === Qt.RightButton) {
                        const p = mapToItem(null, m.x, m.y), d = new Date(cell.date);
                        d.setHours(9);
                        v.slotContext(d, p.x, p.y);
                        return;
                    }
                    v.rangeFrom = cell.index;
                    v.rangeTo = -1;
                    forceActiveFocus();
                }
                onPositionChanged: m => {
                    if (pressed)
                        v.rangeTo = v.indexAt(mapToItem(v, m.x, m.y).x, mapToItem(v, m.x, m.y).y);
                }
                onReleased: {
                    if (v.rangeTo >= 0 && v.rangeTo !== v.rangeFrom)
                        v.daysCreated(v.tools.addDays(v.gridStart, Math.min(v.rangeFrom, v.rangeTo)), v.tools.addDays(v.gridStart, Math.max(v.rangeFrom, v.rangeTo)));
                    v.reset();
                }
                onCanceled: v.reset()
                Keys.onEscapePressed: v.reset()
            }
            // Day number: today is a disc in the accent colour
            Rectangle {
                x: FiliTheme.space1 + 2
                y: FiliTheme.space1
                width: Math.max(22, dayText.implicitWidth + 10)
                height: 22
                radius: height / 2
                color: cell.isToday ? FiliTheme.accentColor : "transparent"
                Text {
                    id: dayText
                    anchors.centerIn: parent
                    text: cell.date.getDate() === 1 ? cell.date.getDate() + " " + Qt.locale().monthName(cell.date.getMonth(), Locale.ShortFormat) : cell.date.getDate()
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.size
                    font.weight: cell.isToday || cell.date.getDate() === 1 ? FiliTheme.weightAccent : Font.Normal
                    color: cell.isToday ? (0.299 * FiliTheme.accentColor.r + 0.587 * FiliTheme.accentColor.g + 0.114 * FiliTheme.accentColor.b > 0.55 ? "#14181f" : "#ffffff") : cell.inMonth ? FiliTheme.text : FiliTheme.muted
                }
            }
            Column {
                anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: 28; leftMargin: 3; rightMargin: 3 }
                spacing: 2
                Repeater {
                    model: cell.dayEvents.slice(0, cell.shown)
                    Rectangle {
                        id: chip
                        required property var modelData
                        readonly property bool whole: modelData.allDay
                        width: parent.width
                        height: 16
                        radius: 4
                        readonly property color tone: v.tools.colorOf(modelData)
                        color: whole ? tone : FiliTheme.alpha(tone, chipArea.containsMouse ? 0.28 : 0.16)
                        opacity: (cell.inMonth ? 1 : 0.55) * (modelData.done ? 0.5 : 1)
                        border.width: v.selectedKey === modelData.id + "|" + modelData.occ ? 1 : 0
                        border.color: FiliTheme.text
                        Rectangle { visible: !chip.whole; width: 3; height: parent.height - 4; y: 2; radius: 1.5; color: chip.tone }
                        JoinButton {
                            tools: v.tools
                            url: chip.modelData.join || ""
                            service: chip.modelData.joinService || ""
                            tone: chip.tone
                            visible: url !== "" && v.cellW > 90
                            width: 24
                            height: 16
                            radius: 8
                            anchors { right: parent.right; rightMargin: 3; verticalCenter: parent.verticalCenter }
                            z: 5
                            onClicked: v.joinRequested(chip.modelData)
                        }
                        Text {
                            anchors { left: parent.left; leftMargin: chip.whole ? 5 : 8; right: parent.right; rightMargin: chip.modelData.join ? 30 : 4; verticalCenter: parent.verticalCenter }
                            text: (chip.whole ? "" : v.tools.clock(chip.modelData) + " ") + chip.modelData.title
                            font.strikeout: chip.modelData.done === true
                            elide: Text.ElideRight
                            font.family: FiliTheme.font
                            font.pixelSize: FiliTheme.sizeSmall
                            color: chip.whole ? v.tools.inkOn(chip.tone) : FiliTheme.text
                        }
                        MouseArea {
                            id: chipArea
                            anchors.fill: parent
                            hoverEnabled: true
                            property point from
                            property bool moving: false
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onPressed: m => {
                                if (m.button === Qt.RightButton) {
                                    const p = mapToItem(null, m.x, m.y);
                                    v.eventContext(chip.modelData, p.x, p.y);
                                    return;
                                }
                                from = mapToItem(v, m.x, m.y);
                                moving = false;
                                forceActiveFocus();
                            }
                            onPositionChanged: m => {
                                // the events of a subscription cannot be moved
                                if (!pressed || chip.modelData.readonly)
                                    return;
                                const p = mapToItem(v, m.x, m.y);
                                if (!moving && Math.hypot(p.x - from.x, p.y - from.y) < 6)
                                    return;
                                moving = true;
                                v.dropIndex = v.indexAt(p.x, p.y);
                            }
                            onReleased: m => {
                                const to = v.dropIndex;
                                v.reset();
                                if (!moving)
                                    v.eventClicked(chip.modelData);
                                else if (to >= 0 && to !== cell.index)
                                    v.eventMoved(chip.modelData, to - cell.index, (m.modifiers & Qt.AltModifier) !== 0);
                                moving = false;
                            }
                            onCanceled: { moving = false; v.reset(); }
                            Keys.onEscapePressed: { moving = false; v.reset(); }
                        }
                        Accessible.role: Accessible.Button
                        Accessible.name: chip.modelData.title
                    }
                }
                Text {
                    visible: cell.dayEvents.length > cell.shown
                    leftPadding: 4
                    text: Fili.tr("+%1 more").replace("%1", cell.dayEvents.length - cell.shown)
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeSmall
                    color: FiliTheme.muted
                    MouseArea { anchors.fill: parent; onClicked: v.moreIndex = cell.index }
                }
            }
        }
    }

    // Week numbers
    Repeater {
        model: v.wkW > 0 ? 6 : 0
        Text {
            required property int index
            x: 0
            y: v.headH + index * v.cellH + FiliTheme.space1 + 3
            width: v.wkW - 6
            horizontalAlignment: Text.AlignRight
            // the week of its Thursday, as the ISO number counts it
            text: v.tools.isoWeek(v.tools.addDays(v.gridStart, index * 7 + ((4 - v.firstDay + 7) % 7)))
            font.family: FiliTheme.mono
            font.pixelSize: FiliTheme.sizeSmall
            color: FiliTheme.muted
            Accessible.name: Fili.tr("Week %1").replace("%1", text)
        }
    }

    // "+n more": every event of the day in a small popover next to its cell
    Item {
        id: more
        visible: v.moreIndex >= 0
        anchors.fill: parent
        z: 10
        readonly property date day: v.tools.addDays(v.gridStart, Math.max(0, v.moreIndex))
        readonly property var list: v.moreIndex >= 0 ? v.tools.eventsOn(v.events, day) : []
        MouseArea { anchors.fill: parent; onClicked: v.moreIndex = -1 }
        Rectangle {
            readonly property int slot: v.colList.indexOf(Math.max(0, v.moreIndex) % 7)
            width: Math.min(260, v.width - 8)
            height: Math.min(v.height - 8, 34 + more.list.length * 22 + 8)
            x: Math.max(4, Math.min(v.width - width - 4, v.wkW + Math.max(0, slot) * v.cellW))
            y: Math.max(4, Math.min(v.height - height - 4, v.headH + Math.floor(Math.max(0, v.moreIndex) / 7) * v.cellH))
            radius: FiliTheme.radiusWindow
            color: FiliTheme.surface
            border.width: FiliTheme.borderWidth
            border.color: FiliTheme.border
            Text {
                x: FiliTheme.space3
                y: FiliTheme.space2
                text: Qt.locale().toString(more.day, "dddd d MMMM")
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                font.weight: FiliTheme.weightAccent
                color: FiliTheme.text
            }
            Column {
                x: FiliTheme.space2
                y: 32
                width: parent.width - 2 * FiliTheme.space2
                spacing: 2
                Repeater {
                    model: more.list
                    Rectangle {
                        id: row
                        required property var modelData
                        width: parent.width
                        height: 20
                        radius: 4
                        color: FiliTheme.alpha(v.tools.colorOf(modelData), rowArea.containsMouse ? 0.3 : 0.18)
                        Rectangle { visible: !modelData.allDay; width: 3; height: parent.height - 4; y: 2; radius: 1.5; color: v.tools.colorOf(modelData) }
                        Text {
                            anchors { left: parent.left; leftMargin: 8; right: parent.right; rightMargin: 4; verticalCenter: parent.verticalCenter }
                            text: (modelData.allDay ? "" : v.tools.clock(modelData) + " ") + modelData.title
                            elide: Text.ElideRight
                            font.family: FiliTheme.font
                            font.pixelSize: FiliTheme.sizeSmall
                            font.strikeout: modelData.done === true
                            color: FiliTheme.text
                        }
                        MouseArea { id: rowArea; anchors.fill: parent; hoverEnabled: true; onClicked: { v.moreIndex = -1; v.eventClicked(row.modelData); } }
                        Accessible.role: Accessible.Button
                        Accessible.name: modelData.title
                    }
                }
            }
        }
    }
}
