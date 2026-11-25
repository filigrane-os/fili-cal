// List: the events of the next days one under the other, grouped by day, the quickest way to read a week. A click opens the event.
import QtQuick
import Fili.Agenda
import Fili.UI

Item {
    id: v
    required property var tools
    property var events: []
    property date today: new Date()
    signal eventClicked(var ev)
    signal eventContext(var ev, real x, real y)
    // The box of a whole-day event (a reminder without a time) was ticked
    signal doneToggled(var ev)
    property string selectedKey: ""

    // [{ day: Date, items: [ev] }] for the days that have something
    readonly property var days: {
        const out = [];
        let cur = null;
        for (const e of events) {
            const s = tools.day(tools.start(e));
            if (cur === null || !tools.sameDay(cur.day, s)) {
                cur = { day: s, items: [] };
                out.push(cur);
            }
            cur.items.push(e);
        }
        return out;
    }
    // Rows: a header for each day, then its events
    readonly property var rows: {
        const out = [];
        for (const g of days) {
            out.push({ head: true, day: g.day });
            for (const e of g.items)
                out.push({ head: false, ev: e, day: g.day });
        }
        return out;
    }

    function dayText(d) {
        const L = Qt.locale();
        const base = L.dayName(d.getDay(), Locale.LongFormat) + " " + d.getDate() + " " + L.monthName(d.getMonth(), Locale.LongFormat);
        return tools.sameDay(d, today) ? Fili.tr("Today") + " · " + base : tools.sameDay(d, tools.addDays(today, 1)) ? Fili.tr("Tomorrow") + " · " + base : base;
    }

    ListView {
        id: list
        anchors { fill: parent; rightMargin: scroll.width }
        clip: true
        model: v.rows
        boundsBehavior: Flickable.StopAtBounds
        KineticScroll {}
        delegate: Item {
            id: row
            required property var modelData
            width: list.width
            height: modelData.head ? 40 : 44
            Text {
                visible: row.modelData.head
                anchors { left: parent.left; leftMargin: FiliTheme.margin; right: parent.right; rightMargin: FiliTheme.margin; bottom: parent.bottom; bottomMargin: 6 }
                text: v.dayText(row.modelData.day)
                elide: Text.ElideRight
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.size
                font.weight: FiliTheme.weightAccent
                color: v.tools.sameDay(row.modelData.day, v.today) ? FiliTheme.accentColor : FiliTheme.text
            }
            Rectangle {
                visible: row.modelData.head
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: FiliTheme.margin; rightMargin: FiliTheme.margin }
                height: FiliTheme.borderWidth
                color: FiliTheme.alpha(FiliTheme.border, 0.6)
            }
            Rectangle {
                visible: !row.modelData.head
                anchors { fill: parent; leftMargin: FiliTheme.space2; rightMargin: FiliTheme.space2; topMargin: 1; bottomMargin: 1 }
                radius: FiliTheme.radiusControl
                color: area.containsMouse || (!row.modelData.head && v.selectedKey === row.modelData.ev.id + "|" + row.modelData.ev.occ) ? FiliTheme.surfaceHover : "transparent"
            }
            Item {
                visible: !row.modelData.head
                anchors.fill: parent
                Rectangle { x: FiliTheme.margin; y: 8; width: 3; height: parent.height - 16; radius: 1.5; color: row.modelData.head ? "transparent" : v.tools.colorOf(row.modelData.ev) }
                // A whole-day event can be ticked as done
                Rectangle {
                    id: tick
                    readonly property bool shown: !row.modelData.head && row.modelData.ev.allDay === true && row.modelData.ev.readonly !== true
                    visible: shown
                    x: FiliTheme.margin + 14
                    anchors.verticalCenter: parent.verticalCenter
                    width: 16
                    height: 16
                    radius: 4
                    color: shown && row.modelData.ev.done ? FiliTheme.accentColor : "transparent"
                    border.width: 2
                    border.color: shown && row.modelData.ev.done ? FiliTheme.accentColor : FiliTheme.muted
                    Text { visible: parent.color !== "transparent"; anchors.centerIn: parent; text: "✓"; font.pixelSize: 11; color: FiliTheme.background }
                    MouseArea { anchors.fill: parent; anchors.margins: -4; onClicked: v.doneToggled(row.modelData.ev) }
                    Accessible.role: Accessible.CheckBox
                    Accessible.name: shown ? row.modelData.ev.title : ""
                    Accessible.checked: shown && row.modelData.ev.done === true
                }
                Text {
                    x: FiliTheme.margin + 14 + (tick.shown ? 24 : 0)
                    width: 70
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.modelData.head ? "" : row.modelData.ev.allDay ? (tick.shown ? "" : Fili.tr("All day")) : v.tools.clock(row.modelData.ev)
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeList
                    color: FiliTheme.textSecondary
                }
                Column {
                    anchors { left: parent.left; leftMargin: FiliTheme.margin + 94 + (tick.shown ? 24 : 0); right: parent.right; rightMargin: FiliTheme.margin; verticalCenter: parent.verticalCenter }
                    Text {
                        width: parent.width
                        text: row.modelData.head ? "" : row.modelData.ev.title
                        font.strikeout: !row.modelData.head && row.modelData.ev.done === true
                        elide: Text.ElideRight
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.size
                        color: FiliTheme.text
                    }
                    Text {
                        visible: !row.modelData.head && row.modelData.ev.place !== ""
                        width: parent.width
                        text: row.modelData.head ? "" : row.modelData.ev.place
                        elide: Text.ElideRight
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.sizeSmall
                        color: FiliTheme.muted
                    }
                }
                MouseArea {
                    id: area
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: m => {
                        if (m.button === Qt.RightButton) { const p = mapToItem(null, m.x, m.y); v.eventContext(row.modelData.ev, p.x, p.y); } else v.eventClicked(row.modelData.ev);
                    }
                }
                Accessible.role: Accessible.ListItem
                Accessible.name: row.modelData.head ? "" : row.modelData.ev.title
            }
        }
    }
    Text {
        visible: v.rows.length === 0
        anchors.centerIn: parent
        text: Fili.tr("Nothing in this period")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.size
        color: FiliTheme.muted
    }
    FiliScrollBar { id: scroll; flick: list; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
}
