// QML interface compiled into the binary (module Fili.Agenda, qrc:/qt/qml/Fili/Agenda/qml/), Rust objects of src/bridge.rs
use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // Tokens of the shared kit (Fili.UI), for the test that checks the FiliTheme.* names used in the QML
    if let Ok(p) = std::env::var("DEP_FILI_UI_THEME_QML") {
        println!("cargo::rustc-env=FILI_UI_THEME_QML={p}");
    }
    // Version of the About card: tag or commit of the repository (absent outside a repository)
    if let Ok(o) = std::process::Command::new("git").args(["describe", "--tags", "--always", "--dirty"]).output()
        && o.status.success()
    {
        println!("cargo:rustc-env=FILI_CAL_GIT={}", String::from_utf8_lossy(&o.stdout).trim());
    }
    CxxQtBuilder::new_qml_module(
        QmlModule::new("Fili.Agenda")
            .qml_files([
                "qml/Main.qml",
                "qml/DateTools.qml",
                "qml/MonthView.qml",
                "qml/TimeGrid.qml",
                "qml/MiniMonth.qml",
                "qml/CalendarDialog.qml",
                "qml/EventEditor.qml",
            ])
            .depend("Fili.UI")
            .depend("QtQuick")
            .depend("QtQuick.Controls"),
    )
    // Fonts of the interface travel with the binary: the application looks the same on any distribution
    .qrc_resources([
        "fonts/Jost-Variable.ttf",
        "fonts/JetBrainsMono-Variable.ttf",
        "fonts/FiliIcons.ttf",
        "icons/fili-cal.svg",
    ])
    .qt_module("Quick")
    .files(["src/bridge.rs"])
    .build();
}
