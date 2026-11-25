// The button that joins a video conference: a small camera on the accent colour of the calendar, the name of the service in its
// tooltip name. `url` is the meeting link (the owner opens it); `service` is Teams, Meet, Zoom… A click joins without selecting the event.
import QtQuick
import QtQuick.Shapes
import Fili.Agenda
import Fili.UI

Rectangle {
    id: b
    required property var tools
    property string url
    property string service
    property color tone: FiliTheme.accentColor
    // With a label (the name of the service) next to the camera, for lists and the editor
    property bool labelled: false
    signal clicked()

    visible: url !== ""
    implicitWidth: labelled ? 24 + label.implicitWidth + FiliTheme.space2 : 22
    implicitHeight: 20
    radius: 10
    color: area.pressed ? Qt.darker(tone, 1.2) : area.containsMouse ? Qt.lighter(tone, 1.15) : tone
    Shape {
        x: 4
        y: 3
        width: 24
        height: 24
        scale: 14 / 24
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: b.tools.inkOn(b.tone)
            strokeWidth: 2.4
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: "m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5 M4 6h10a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z" }
        }
    }
    Text {
        id: label
        visible: b.labelled
        anchors { left: parent.left; leftMargin: 22; verticalCenter: parent.verticalCenter }
        text: b.service !== "" ? Fili.tr("Join %1").replace("%1", b.service) : Fili.tr("Join")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        font.weight: FiliTheme.weightAccent
        color: b.tools.inkOn(b.tone)
    }
    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: b.clicked()
    }
    Accessible.role: Accessible.Button
    Accessible.name: service !== "" ? Fili.tr("Join %1").replace("%1", service) : Fili.tr("Join")
}
