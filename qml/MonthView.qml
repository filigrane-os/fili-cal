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
    readonly property real cellW: width / 7
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
        return Math.max(0, Math.min(5, Math.floor((y - headH) / cellH))) * 7 + Math.max(0, Math.min(6, Math.floor(x / cellW)));
    }
    function reset() { dropIndex = -1; rangeFrom = -1; rangeTo = -1; }

    // Weekday names
    Row {
        height: v.headH
        Repeater {
            model: 7
            Item {
                required property int index
                width: v.cellW
                height: v.headH
                Text {
                    anchors { left: parent.left; leftMargin: FiliTheme.space2 + 2; verticalCenter: parent.verticalCenter }
                    text: Qt.locale().dayName((v.firstDay + index) % 7, Locale.ShortFormat).toUpperCase()
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
            x: (index % 7) * v.cellW
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
                onPressed: { v.rangeFrom = cell.index; v.rangeTo = -1; forceActiveFocus(); }
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
                        opacity: cell.inMonth ? 1 : 0.55
                        Rectangle { visible: !chip.whole; width: 3; height: parent.height - 4; y: 2; radius: 1.5; color: chip.tone }
                        Text {
                            anchors { left: parent.left; leftMargin: chip.whole ? 5 : 8; right: parent.right; rightMargin: 4; verticalCenter: parent.verticalCenter }
                            text: (chip.whole ? "" : v.tools.clock(chip.modelData) + " ") + chip.modelData.title
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
                            onPressed: m => { from = mapToItem(v, m.x, m.y); moving = false; forceActiveFocus(); }
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
                }
            }
        }
    }
}
