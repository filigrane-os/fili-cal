// Add an account: a type (Nextcloud, Fastmail, iCloud, another CalDAV server, or the link of a calendar: Proton, Google, any iCalendar
// link), then the form of that type. `connect(address, user, password)` (a CalDAV account, kept in step with the server) and
// `subscribe(name, link)` (a read-only calendar fetched from a link) are for the owner to apply. The password is not kept here: it
// goes to the keyring.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    signal connect(string address, string user, string password)
    signal subscribe(string name, string link)

    // The kinds: `dav` is a CalDAV account (address, user, password), the others a link. `host` is the fixed server of the service.
    readonly property var kinds: [
        { value: "nextcloud", label: "Nextcloud", dav: true, host: "", suffix: "/remote.php/dav/", addressLabel: Fili.tr("Server address"), addressHint: "https://cloud.example.org", userLabel: Fili.tr("User name"), help: Fili.tr("Use an app password: Settings, Security, Devices & sessions.") },
        { value: "fastmail", label: "Fastmail", dav: true, host: "https://caldav.fastmail.com/", userLabel: Fili.tr("E-mail address"), help: Fili.tr("Use an app password with access to calendars: Settings, Privacy & Security, Integrations.") },
        { value: "icloud", label: "iCloud", dav: true, host: "https://caldav.icloud.com/", userLabel: Fili.tr("Apple ID"), help: Fili.tr("Use an app-specific password made at appleid.apple.com, not the one of your Apple ID.") },
        { value: "caldav", label: Fili.tr("Other CalDAV server"), dav: true, host: "", suffix: "", addressLabel: Fili.tr("Address of the server or of the calendar"), addressHint: "https://", userLabel: Fili.tr("User name"), help: Fili.tr("The address of the server is enough: the calendars are found from it.") },
        { value: "proton", label: "Proton Calendar", dav: false, nameHint: "Proton Calendar", help: Fili.tr("In Proton Calendar, share a calendar with anyone and copy its link. The calendar is read-only here.") },
        { value: "google", label: "Google Calendar", dav: false, nameHint: "Google Calendar", help: Fili.tr("In Google Calendar, Settings of the calendar, Integrate calendar, Secret address in iCal format. The calendar is read-only here.") },
        { value: "link", label: Fili.tr("Link to a calendar (iCalendar)"), dav: false, nameHint: Fili.tr("Name"), help: Fili.tr("An https:// or webcal:// link makes a read-only calendar that is fetched now and then.") }
    ]
    property string kind: "nextcloud"
    readonly property var current: kinds.find(k => k.value === kind) || kinds[0]
    readonly property bool needsAddress: current.dav && current.host === ""
    readonly property bool formOk: current.dav
        ? (!needsAddress || /^https?:\/\/\S+$/.test(addressField.text.trim())) && userField.text.trim() !== "" && passwordField.text !== ""
        : nameField.text.trim() !== "" && /^(https?|webcals?):\/\/\S+$/.test(linkField.text.trim())

    title: Fili.tr("Add account")
    confirmLabel: current.dav ? Fili.tr("Connect") : Fili.tr("Subscribe")
    cardWidth: 440
    confirmEnabled: formOk

    function start() {
        kind = "nextcloud";
        addressField.text = "";
        userField.text = "";
        passwordField.text = "";
        nameField.text = "";
        linkField.text = "";
        open();
        addressField.focusField();
    }
    onAccepted: {
        if (current.dav) {
            const address = current.host !== "" ? current.host : addressField.text.trim().replace(/\/+$/, "") + (current.suffix || "");
            connect(address, userField.text.trim(), passwordField.text);
        } else {
            subscribe(nameField.text.trim(), linkField.text.trim());
        }
        passwordField.text = "";
    }

    Text {
        text: Fili.tr("Account type")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    FiliSelect {
        width: parent.width
        model: dlg.kinds.map(k => ({ label: k.label, value: k.value }))
        current: dlg.kind
        onPicked: v => { dlg.kind = v; if (dlg.current.dav) { if (dlg.needsAddress) addressField.focusField(); else userField.focusField(); } else nameField.focusField(); }
    }
    // A CalDAV account
    FiliTextField { id: addressField; visible: dlg.needsAddress; width: parent.width; placeholder: dlg.current.addressLabel + (dlg.current.addressHint ? " (" + dlg.current.addressHint + ")" : ""); onAccepted: dlg.accept() }
    FiliTextField { id: userField; visible: dlg.current.dav; width: parent.width; placeholder: dlg.current.userLabel || ""; onAccepted: dlg.accept() }
    FiliTextField { id: passwordField; visible: dlg.current.dav; width: parent.width; secret: true; placeholder: Fili.tr("Password"); onAccepted: dlg.accept() }
    // A link
    FiliTextField { id: nameField; visible: !dlg.current.dav; width: parent.width; placeholder: dlg.current.nameHint || Fili.tr("Name"); onAccepted: dlg.accept() }
    FiliTextField { id: linkField; visible: !dlg.current.dav; width: parent.width; placeholder: Fili.tr("Link (https:// or webcal://)"); onAccepted: dlg.accept() }
    Text {
        width: parent.width
        wrapMode: Text.WordWrap
        text: dlg.current.help + (dlg.current.dav ? " " + Fili.tr("The password is kept in the keyring.") : "")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
}
