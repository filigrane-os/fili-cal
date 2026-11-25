// fili-cal: Agenda (aka k7), the sound recorder and player of Filigrane. Without arguments it opens the window
// (Qt Quick, like Casier); with a command it works without one: start, stop, toggle, status, set, folder (the fili-bar
// module and scripts use them), ctl (commands to an open window).
use fili_ui as _;

mod bridge;
mod calendars;
mod context;
mod cli;
mod ctl;
mod dav;
mod exchange;
mod i18n;
mod ics;
mod logs;
mod model;
mod settings;
mod subscribe;
mod theme;
mod undo;
mod when;
mod window;
mod xml;
mod zone;

pub use cli::short_hash;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("-V" | "--version") => {
            println!("fili-cal {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("-h" | "--help") => {
            println!(
                "fili-cal {}: Agenda, the calendar and the reminders of Filigrane\n\nUsage: fili-cal [--no-header] [--no-footer]   open the window\n       fili-cal add|quick|edit|rm|show|list|next|tick|calendar|import|export ...   (fili-cal help)\n       fili-cal ctl [--pid PID] action NAME [ARG] | state | actions",
                env!("CARGO_PKG_VERSION")
            );
            return;
        }
        Some("add" | "quick" | "edit" | "rm" | "show" | "list" | "next" | "tick" | "free" | "calendar" | "import" | "export" | "undo" | "redo" | "alert-wait" | "help") => {
            match cli::run(&args[1..]) {
                Ok(()) => return,
                Err(e) => {
                    eprintln!("fili-cal: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("ctl") => {
            match ctl::client(&args[2..]) {
                Ok(answer) => println!("{answer}"),
                Err(e) => {
                    eprintln!("fili-cal ctl: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        _ => {}
    }
    // A file or a link to open (what the system gives a handler of text/calendar and webcal:): the open window shows its import
    // preview, or the new one does
    // (the value of --shot or --view is not a file to open)
    if let Some(src) = args.iter().enumerate().skip(1).filter(|(i, _)| !matches!(args[i - 1].as_str(), "--shot" | "--view")).map(|(_, a)| a).find(|a| !a.starts_with("--") && (a.starts_with("webcal:") || a.starts_with("webcals:") || a.starts_with("file:") || a.starts_with("http") || std::path::Path::new(a).is_file())) {
        let abs = if src.contains("://") || src.starts_with("webcal") { src.clone() } else { std::fs::canonicalize(src).map_or_else(|_| src.clone(), |p| p.display().to_string()) };
        if let Some(pid) = ctl::windows().first()
            && ctl::client(&["--pid".to_string(), pid.to_string(), "action".to_string(), "import-file".to_string(), abs.clone()]).is_ok()
        {
            window::focus_pid(*pid);
            return;
        }
        // SAFETY: nothing else runs yet, the application is not started
        unsafe { std::env::set_var("FILI_CAL_OPEN", &abs) };
    }
    // One window: a second launch brings the open one to the front
    if let Some(pid) = ctl::windows().first()
        && window::focus_pid(*pid)
    {
        return;
    }
    // The log of the window (Debug panel, View > Debug Logs): ~/.local/state/filigrane/logs/fili-cal.log, panics and
    // QML errors become incidents (fili-incident)
    logs::init("fili-cal", "filigrane-os/filigrane", env!("CARGO_PKG_VERSION"), theme::incidents_on());
    logs::capture_qt();
    let mut app = QGuiApplication::new();
    // Wayland app-id (niri rules, fili-cal.desktop)
    QGuiApplication::set_desktop_file_name(&QString::from("fili-cal"));
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        // Development (`make run`): the QML is read from the source folders instead of the binary, so a change of a .qml file
        // needs no build. FILI_QML_DIR holds Main.qml; FILI_UI_QML the kit's folder, searched before the compiled-in Fili.UI.
        if let Ok(dir) = std::env::var("FILI_UI_QML") {
            engine.as_mut().add_import_path(&QString::from(&dir));
        }
        match std::env::var("FILI_QML_DIR") {
            Ok(dir) => engine.load(&QUrl::from(&format!("file://{dir}/Main.qml"))),
            Err(_) => engine.load(&QUrl::from("qrc:/qt/qml/Fili/Agenda/qml/Main.qml")),
        }
    }
    if let Some(app) = app.as_mut() {
        let code = app.exec();
        context::clear();
        std::process::exit(code);
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    fn read(rel: &str) -> String {
        fs::read_to_string(format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    // A misspelled token does not fail at run time: the item just stays transparent
    #[test]
    fn every_theme_token_used_in_qml_is_defined() {
        let defined: Vec<String> = fs::read_to_string(env!("FILI_UI_THEME_QML")).unwrap()
            .lines()
            .filter_map(|l| l.trim().strip_prefix("readonly property "))
            .filter_map(|l| Some(l.split(':').next()?.split_whitespace().last()?.to_string()))
            .chain(["alpha".to_string()])
            .collect();
        for entry in fs::read_dir(format!("{}/qml", env!("CARGO_MANIFEST_DIR"))).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "FiliTheme.qml") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for (i, _) in text.match_indices("FiliTheme.") {
                let name: String = text[i + 10..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                assert!(defined.contains(&name), "{}: FiliTheme.{name} is not defined", path.display());
            }
        }
    }

    // A function the QML calls on Fili or cas but that is not exposed (#[qinvokable]) is a run-time TypeError that no
    // other test sees: the button just does nothing
    #[test]
    fn every_function_called_by_the_qml_is_exposed() {
        let bridge = read("src/bridge.rs");
        let lines: Vec<&str> = bridge.lines().map(str::trim).collect();
        // name used from QML -> exposed or not, for every `fn` declared in the bridge's extern block
        let mut exposed: HashMap<String, bool> = HashMap::new();
        for (i, l) in lines.iter().enumerate() {
            // Declarations of the extern block end with ";" (the functions of the impl blocks end with "{")
            let Some(rest) = l.strip_prefix("fn ").filter(|_| l.ends_with(';')) else { continue };
            let rust = rest.split('(').next().unwrap_or("");
            let mut attrs = Vec::new();
            let mut j = i;
            while j > 0 && lines[j - 1].starts_with("#[") {
                j -= 1;
                attrs.push(lines[j]);
            }
            let cxx = attrs.iter().find_map(|a| a.strip_prefix("#[cxx_name = \"").and_then(|r| r.strip_suffix("\"]")));
            let camel = {
                let mut out = String::new();
                let mut up = false;
                for c in rust.chars() {
                    if c == '_' {
                        up = true
                    } else if up {
                        out.extend(c.to_uppercase());
                        up = false
                    } else {
                        out.push(c)
                    }
                }
                out
            };
            let ok = attrs.iter().any(|a| *a == "#[qinvokable]" || a.starts_with("#[qsignal"));
            exposed.insert(cxx.map(String::from).unwrap_or(camel), ok);
        }
        for entry in fs::read_dir(format!("{}/qml", env!("CARGO_MANIFEST_DIR"))).unwrap() {
            let path = entry.unwrap().path();
            let text = fs::read_to_string(&path).unwrap();
            for obj in ["Fili.", "cal."] {
                for (i, _) in text.match_indices(obj) {
                    let name: String = text[i + obj.len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    if text[i + obj.len() + name.len()..].starts_with('(') {
                        if let Some(false) = exposed.get(&name) {
                            panic!("{}: {obj}{name}() is called but not exposed to QML (missing #[qinvokable])", path.display());
                        }
                    }
                }
            }
        }
    }

    // A singleton that is not registered in build.rs is "not defined" at run time, which no other test would notice
    #[test]
    fn every_qml_singleton_is_registered() {
        let build = read("build.rs");
        for entry in fs::read_dir(format!("{}/qml", env!("CARGO_MANIFEST_DIR"))).unwrap() {
            let path = entry.unwrap().path();
            if fs::read_to_string(&path).unwrap().contains("pragma Singleton") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                assert!(
                    build.contains(&format!("\"qml/{name}\").singleton(true)")),
                    "{name} is not registered as a singleton in build.rs"
                );
            }
        }
    }

    // A QML file that build.rs does not list is "type not found" at run time
    #[test]
    fn every_qml_file_is_listed_in_build_rs() {
        let build = read("build.rs");
        for entry in fs::read_dir(format!("{}/qml", env!("CARGO_MANIFEST_DIR"))).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            assert!(build.contains(&format!("\"qml/{name}\"")), "{name} is not listed in build.rs");
        }
    }
}
