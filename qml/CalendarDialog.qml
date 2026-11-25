// A calendar: its name and its colour (one of the accent tones of the theme), and Delete when it exists. `create()` opens it on a
// new calendar, `edit(c)` on one of the list ({slug, name, color, hidden}). `saved(slug, name, color)` (slug empty = new) and
// `deleted(slug)` are for the owner to apply.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    required property var tools
    property string slug: ""
    property string color: "default"
    readonly property bool isNew: slug === ""
    readonly property var tones: ["default", "voile", "ardoise", "sauge", "sable", "cendre", "brume"]
    signal saved(string slug, string name, string color)
    signal deleted(string slug)

    title: isNew ? Fili.tr("New calendar") : Fili.tr("Calendar")
    confirmLabel: Fili.tr("Save")
    cardWidth: 380
    confirmEnabled: nameField.text.trim() !== ""

    function create() {
        slug = "";
        color = tones[1 + Math.floor(Math.random() * (tones.length - 1))];
        nameField.text = "";
        open();
        nameField.focusField();
    }
    function edit(c) {
        slug = c.slug;
        color = c.color;
        nameField.text = c.name;
        open();
        nameField.focusField();
    }
    onAccepted: saved(slug, nameField.text.trim(), color)

    FiliTextField { id: nameField; width: parent.width; placeholder: Fili.tr("Name"); onAccepted: dlg.accept() }
    Row {
        spacing: FiliTheme.space2
        Repeater {
            model: dlg.tones
            Rectangle {
                id: sw
                required property string modelData
                width: 26
                height: 26
                radius: 13
                color: dlg.tools.colorOf({ color: modelData })
                border.width: dlg.color === modelData ? 2 : 0
                border.color: FiliTheme.text
                MouseArea { anchors.fill: parent; onClicked: dlg.color = sw.modelData }
                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData
                Accessible.checked: dlg.color === modelData
            }
        }
    }
    Row {
        visible: !dlg.isNew && dlg.slug !== "personal"
        FiliButton { label: Fili.tr("Delete calendar"); onClicked: { dlg.close(); dlg.deleted(dlg.slug); } }
    }
}
