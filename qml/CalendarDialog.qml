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
    property bool subscribed: false
    readonly property bool isNew: slug === ""
    readonly property var tones: ["default", "voile", "ardoise", "sauge", "sable", "cendre", "brume"]
    property string status: ""
    signal saved(string slug, string name, string color, string url)
    signal refreshRequested()
    signal deleted(string slug)

    title: isNew ? Fili.tr("New calendar") : Fili.tr("Calendar")
    confirmLabel: Fili.tr("Save")
    cardWidth: 380
    confirmEnabled: nameField.text.trim() !== ""

    function create() {
        slug = "";
        status = "";
        urlField.text = "";
        color = tones[1 + Math.floor(Math.random() * (tones.length - 1))];
        nameField.text = "";
        open();
        nameField.focusField();
    }
    function edit(c) {
        slug = c.slug;
        color = c.color;
        urlField.text = "";
        status = !c.subscribed ? "" : c.error !== "" ? Fili.tr("Last fetch failed: %1").replace("%1", c.error) : c.fetched > 0 ? Fili.tr("Updated %1").replace("%1", new Date(c.fetched * 1000).toLocaleString(Qt.locale(), Locale.ShortFormat)) : Fili.tr("Not fetched yet");
        subscribed = c.subscribed;
        nameField.text = c.name;
        open();
        nameField.focusField();
    }
    onAccepted: saved(slug, nameField.text.trim(), color, urlField.text.trim())

    FiliTextField { id: nameField; width: parent.width; placeholder: Fili.tr("Name"); onAccepted: dlg.accept() }
    FiliTextField { id: urlField; visible: dlg.isNew; width: parent.width; placeholder: Fili.tr("Link to subscribe to (optional)"); onAccepted: dlg.accept() }
    Text {
        visible: dlg.isNew
        width: parent.width
        wrapMode: Text.WordWrap
        text: Fili.tr("A link (https:// or webcal://) makes a read-only calendar that is fetched now and then.")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    Text {
        visible: dlg.status !== ""
        width: parent.width
        wrapMode: Text.WordWrap
        text: dlg.status
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
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
        spacing: FiliTheme.space2
        FiliButton { visible: dlg.subscribed; label: Fili.tr("Refresh now"); onClicked: { dlg.close(); dlg.refreshRequested(); } }
        FiliButton { label: Fili.tr("Delete calendar"); onClicked: { dlg.close(); dlg.deleted(dlg.slug); } }
    }
}
