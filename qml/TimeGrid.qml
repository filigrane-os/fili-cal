// Day and week: a header with the days, a strip for the whole-day events, then the hours (scrolling) with the events as blocks
// placed by time, side by side when they overlap, and a line at the current time. A double click on an hour opens a new event
// there, a click on a block opens it. Dragging on empty hours draws a new event in steps of 15 minutes; dragging a block moves it
// (to another hour or day, Alt copies it), dragging its bottom edge changes its length; Escape cancels what is being dragged.
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
    // A range dragged on empty hours: new event from `start` to `end`
    signal rangeCreated(date start, date end)
    // A block moved or resized: the minutes to add to its start and to its end (a day is 1440), and whether it is a copy
    signal eventChanged(var ev, int dStart, int dEnd, bool copy)
    readonly property real pad: 6
    // Minutes since midnight at a y of the hours, in steps of 15
    function snap(y) { return Math.max(0, Math.min(1440, Math.round((y - pad) / hourH * 4) * 15)); }
    function colAt(x) { return Math.max(0, Math.min(days - 1, Math.floor((x - gutter) / colW))); }
    function clockText(min) { return tools.hhmm(Math.floor(min / 60) % 24, min % 60); }
    // What is being dragged: the box shown under the pointer
    property var ghost: null

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
                            color: v.tools.colorOf(wc.modelData)
                            Text {
                                anchors { left: parent.left; leftMargin: 5; right: parent.right; rightMargin: 4; verticalCenter: parent.verticalCenter }
                                text: wc.modelData.title
                                elide: Text.ElideRight
                                font.family: FiliTheme.font
                                font.pixelSize: FiliTheme.sizeSmall
                                color: v.tools.inkOn(wc.color)
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
                    id: slotArea
                    anchors.fill: parent
                    property real fromY: 0
                    property bool dragging: false
                    function range(y) { const a = v.snap(slotArea.fromY + v.pad), b = v.snap(y + v.pad); return a <= b ? [a, Math.max(b, a + 15)] : [b, a]; }
                    onPressed: m => { fromY = m.y; dragging = false; forceActiveFocus(); }
                    onPositionChanged: m => {
                        if (!pressed)
                            return;
                        if (!dragging && Math.abs(m.y - fromY) < 8)
                            return;
                        dragging = true;
                        const r = range(m.y);
                        v.ghost = { col: col.index, s: r[0], e: r[1], label: v.clockText(r[0]) + " – " + v.clockText(r[1]), copy: false };
                    }
                    onReleased: m => {
                        if (dragging) {
                            const r = range(m.y), d = v.tools.day(col.modelData), e = new Date(d);
                            d.setMinutes(r[0]);
                            e.setMinutes(r[1]);
                            v.ghost = null;
                            v.rangeCreated(d, e);
                        }
                        dragging = false;
                    }
                    onCanceled: { dragging = false; v.ghost = null; }
                    Keys.onEscapePressed: { dragging = false; v.ghost = null; }
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
                        color: FiliTheme.alpha(v.tools.colorOf(modelData.ev), blkArea.containsMouse ? 0.3 : 0.18)
                        clip: true
                        Rectangle { width: 3; height: parent.height - 4; y: 2; x: 2; radius: 1.5; color: v.tools.colorOf(blk.modelData.ev) }
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
                        MouseArea {
                            id: blkArea
                            anchors.fill: parent
                            hoverEnabled: true
                            property point from
                            property bool moving: false
                            function content(m) { return mapToItem(flick.contentItem, m.x, m.y); }
                            onPressed: m => { from = content(m); moving = false; forceActiveFocus(); }
                            onPositionChanged: m => {
                                if (!pressed)
                                    return;
                                const p = content(m);
                                if (!moving && Math.hypot(p.x - from.x, p.y - from.y) < 6)
                                    return;
                                moving = true;
                                const len = blk.modelData.e - blk.modelData.s;
                                const dMin = Math.round((p.y - from.y) / v.hourH * 4) * 15;
                                const s = Math.max(0, Math.min(1440 - len, blk.modelData.s + dMin));
                                v.ghost = { col: v.colAt(p.x), s: s, e: s + len, label: v.clockText(s) + " – " + v.clockText(s + len), copy: (m.modifiers & Qt.AltModifier) !== 0, title: blk.modelData.ev.title };
                            }
                            onReleased: m => {
                                const g = v.ghost;
                                v.ghost = null;
                                if (!moving) {
                                    v.eventClicked(blk.modelData.ev);
                                } else if (g) {
                                    const d = (g.col - col.index) * 1440 + (g.s - blk.modelData.s);
                                    if (d !== 0)
                                        v.eventChanged(blk.modelData.ev, d, d, g.copy);
                                }
                                moving = false;
                            }
                            onCanceled: { moving = false; v.ghost = null; }
                            Keys.onEscapePressed: { moving = false; v.ghost = null; }
                        }
                        // The bottom edge: dragging it changes the length (a reminder has none)
                        MouseArea {
                            visible: !blk.reminder
                            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                            height: 7
                            cursorShape: Qt.SizeVerCursor
                            property real fromY: 0
                            property bool sizing: false
                            onPressed: m => { fromY = mapToItem(flick.contentItem, m.x, m.y).y; sizing = true; forceActiveFocus(); }
                            onPositionChanged: m => {
                                if (!sizing)
                                    return;
                                const y = mapToItem(flick.contentItem, m.x, m.y).y;
                                const e = Math.max(blk.modelData.s + 15, Math.min(1440, blk.modelData.e + Math.round((y - fromY) / v.hourH * 4) * 15));
                                v.ghost = { col: col.index, s: blk.modelData.s, e: e, label: v.clockText(blk.modelData.s) + " – " + v.clockText(e), copy: false, title: blk.modelData.ev.title };
                            }
                            onReleased: {
                                const g = v.ghost;
                                v.ghost = null;
                                sizing = false;
                                if (g && g.e !== blk.modelData.e)
                                    v.eventChanged(blk.modelData.ev, 0, g.e - blk.modelData.e, false);
                            }
                            onCanceled: { sizing = false; v.ghost = null; }
                            Keys.onEscapePressed: { sizing = false; v.ghost = null; }
                        }
                        Accessible.role: Accessible.Button
                        Accessible.name: blk.modelData.ev.title
                    }
                }
            }
        }
        // What is being dragged
        Rectangle {
            visible: v.ghost !== null
            x: v.gutter + (v.ghost ? v.ghost.col : 0) * v.colW + 2
            y: v.pad + (v.ghost ? v.ghost.s : 0) / 60 * v.hourH + 1
            width: v.colW - 4
            height: v.ghost ? Math.max(20, (v.ghost.e - v.ghost.s) / 60 * v.hourH - 2) : 0
            radius: 5
            color: FiliTheme.alpha(FiliTheme.accentColor, 0.35)
            border.color: FiliTheme.accentColor
            border.width: 1
            Column {
                anchors { left: parent.left; leftMargin: 9; right: parent.right; rightMargin: 4; top: parent.top; topMargin: 3 }
                Text {
                    visible: v.ghost && v.ghost.title
                    width: parent.width
                    text: (v.ghost && v.ghost.copy ? "+ " : "") + (v.ghost ? v.ghost.title || "" : "")
                    elide: Text.ElideRight
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeList
                    font.weight: FiliTheme.weightAccent
                    color: FiliTheme.text
                }
                Text {
                    width: parent.width
                    text: v.ghost ? v.ghost.label : ""
                    elide: Text.ElideRight
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeSmall
                    color: FiliTheme.textSecondary
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
