// A CalDAV account: the address of the server (or of one calendar), the user name and the password. `start()` opens it; `connect(address,
// user, password)` is for the owner to apply. The password is not kept here: it goes to the keyring.
import QtQuick
import Fili.Agenda
import Fili.UI

FiliDialog {
    id: dlg
    signal connect(string address, string user, string password)

    title: Fili.tr("Add account")
    confirmLabel: Fili.tr("Connect")
    cardWidth: 420
    confirmEnabled: /^https?:\/\/\S+$/.test(addressField.text.trim()) && userField.text.trim() !== "" && passwordField.text !== ""

    function start() {
        addressField.text = "";
        userField.text = "";
        passwordField.text = "";
        open();
        addressField.focusField();
    }
    onAccepted: {
        connect(addressField.text.trim(), userField.text.trim(), passwordField.text);
        passwordField.text = "";
    }

    Text {
        width: parent.width
        wrapMode: Text.WordWrap
        text: Fili.tr("Nextcloud, Fastmail, iCloud or any CalDAV server. Use an app password where the service has them. The password is kept in the keyring.")
        font.family: FiliTheme.font
        font.pixelSize: FiliTheme.sizeSmall
        color: FiliTheme.muted
    }
    FiliTextField { id: addressField; width: parent.width; placeholder: Fili.tr("Address of the server or of the calendar"); onAccepted: dlg.accept() }
    FiliTextField { id: userField; width: parent.width; placeholder: Fili.tr("User name"); onAccepted: dlg.accept() }
    FiliTextField { id: passwordField; width: parent.width; secret: true; placeholder: Fili.tr("Password"); onAccepted: dlg.accept() }
}
