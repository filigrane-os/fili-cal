// Objects exposed to QML (module Fili.Agenda): Fili, the environment (live theme, translations, window management, like
// Casier's), and Agenda, the recordings, the player and the recorder of the window.
#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, light, READ, NOTIFY = theme_changed)]
        #[qproperty(QString, accent, READ, NOTIFY = theme_changed)]
        // Counts the changes of the settings files (fili-cal.toml, theme.toml): bindings that read it, and the
        // headless header (Main.qml, onThemeChanged), follow a change made while the window is open
        #[qproperty(i32, settings_rev, cxx_name = "settingsRev", READ, NOTIFY = theme_changed)]
        #[qproperty(bool, key_nav, cxx_name = "keyNav")]
        type Fili = super::FiliRust;

        #[qsignal]
        #[cxx_name = "themeChanged"]
        fn theme_changed(self: Pin<&mut Fili>);
        // Command of `fili-cal ctl` (the menu of the fili-bar, the assistant): number to reply with, verb, argument
        #[qsignal]
        fn command(self: Pin<&mut Fili>, id: i32, verb: &QString, arg: &QString);
        // Answer to a command (see Main.qml, onCommand)
        #[qinvokable]
        fn reply(self: &Fili, id: i32, answer: &QString);

        #[qinvokable]
        #[cxx_name = "themeJson"]
        fn theme_json(self: &Fili) -> QString;
        #[qinvokable]
        fn tr(self: &Fili, text: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "windowControl"]
        fn window_control(self: &Fili) -> bool;
        #[qinvokable]
        #[cxx_name = "windowState"]
        fn window_state(self: &Fili) -> QString;
        #[qinvokable]
        fn header(self: &Fili) -> bool;
        #[qinvokable]
        fn footer(self: &Fili) -> bool;
        #[qinvokable]
        fn attention(self: &Fili) -> bool;
        #[qinvokable]
        #[cxx_name = "windowButtons"]
        fn window_buttons(self: &Fili) -> bool;
        #[qinvokable]
        #[cxx_name = "setWindowButtons"]
        fn set_window_buttons(self: &Fili, on: bool);
        #[qinvokable]
        #[cxx_name = "windowAction"]
        fn window_action(self: &Fili, name: &QString) -> bool;
        // Small interface state kept between two launches, value as text, empty when absent
        #[qinvokable]
        #[cxx_name = "uiState"]
        fn ui_state(self: &Fili, key: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "setUiState"]
        fn set_ui_state(self: &Fili, key: &QString, value: &QString);
        // About card: rows [[label, value]] as JSON
        #[qinvokable]
        #[cxx_name = "aboutRows"]
        fn about_rows(self: &Fili) -> QString;
        #[qinvokable]
        #[cxx_name = "copyText"]
        fn copy_text(self: &Fili, text: &QString);
        // Debug panel (src/logs.rs): the log entries after number `since` as JSON {"seq": last number, "entries": [[level,
        // text], ...]} (level 0 debug, 1 info, 2 warning, 3 error), the file of the log, a way to empty the panel, the file
        // opened by its application, and "Ask Lia": records an incident and asks Lia to look at it (its id, "" on failure)
        #[qinvokable]
        fn logs(self: &Fili, since: i32) -> QString;
        #[qinvokable]
        #[cxx_name = "logFile"]
        fn log_file(self: &Fili) -> QString;
        #[qinvokable]
        #[cxx_name = "clearLogs"]
        fn clear_logs(self: &Fili);
        #[qinvokable]
        #[cxx_name = "openPath"]
        fn open_path(self: &Fili, path: &QString);
        // "Ask Lia" header button: `fili-ai here` shows the small field of Lia scoped to this application; the button
        // shows only where the assistant (`fili-ai-daemon`) is installed
        #[qinvokable]
        #[cxx_name = "askLia"]
        fn ask_lia(self: &Fili);
        #[qinvokable]
        #[cxx_name = "liaAvailable"]
        fn lia_available(self: &Fili) -> bool;
        #[qinvokable]
        #[cxx_name = "reportProblem"]
        fn report_problem(self: &Fili, text: &QString) -> QString;
        // Opens the options file in the default editor
        #[qinvokable]
        #[cxx_name = "openSettings"]
        fn open_settings(self: &Fili);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        // The events of the loaded range as JSON: [{id, title, start, end, allDay, place, note, alarms, every, interval, count,
        // until, occ}] where `start` / `end` are those of the occurrence (YYYY-MM-DDTHH:MM) and `occ` is its start
        #[qproperty(QString, events, READ, NOTIFY = changed)]
        // Result of the last action: the error, empty when it worked
        #[qproperty(QString, message, READ, NOTIFY = changed)]
        #[qproperty(bool, failed, READ, NOTIFY = changed)]
        type Agenda = super::AgendaRust;

        #[qsignal]
        fn changed(self: Pin<&mut Agenda>);

        // Loads the occurrences between two dates (YYYY-MM-DD, `to` excluded)
        #[qinvokable]
        fn load(self: Pin<&mut Agenda>, from: &QString, to: &QString);
        // Reloads the range that is shown (the files changed)
        #[qinvokable]
        fn refresh(self: Pin<&mut Agenda>);
        // Adds an event described as JSON ({title, start, end?, allDay?, place?, note?, alarms?, every?, interval?, count?,
        // until?}); returns its id, empty when it failed (see message)
        #[qinvokable]
        fn add(self: Pin<&mut Agenda>, json: &QString) -> QString;
        // Changes an event: the fields present in the JSON replace those of the event
        #[qinvokable]
        fn update(self: Pin<&mut Agenda>, id: &QString, json: &QString) -> bool;
        // Deletes an event, or only its occurrence starting at `occ` (YYYY-MM-DDTHH:MM) when it is not empty
        #[qinvokable]
        fn remove(self: Pin<&mut Agenda>, id: &QString, occ: &QString) -> bool;
        // Takes the last change back (Ctrl+Z) or does it again (Ctrl+Shift+Z); false when there is nothing to do (see message)
        // The calendars as JSON [{slug, name, color, hidden}]; the colour is the name of an accent tone of the theme
        #[qinvokable]
        fn calendars(self: &Agenda) -> QString;
        // Creates a calendar; returns its slug, empty when it failed (see message)
        #[qinvokable]
        #[cxx_name = "calendarAdd"]
        fn calendar_add(self: Pin<&mut Agenda>, name: &QString, color: &QString, url: &QString) -> QString;
        // What importing a file or a link would add, as JSON {ok, error?, count, events: [{title, start, allDay, place, every}]}
        #[qinvokable]
        #[cxx_name = "previewImport"]
        fn preview_import(self: &Agenda, source: &QString) -> QString;
        // Adds the events of a file or link to a calendar (slug); false when it failed (see message)
        #[qinvokable]
        #[cxx_name = "importSource"]
        fn import_source(self: Pin<&mut Agenda>, source: &QString, calendar: &QString) -> bool;
        // Writes the events of a calendar (slug, "all" = every one) to a file or into a folder; false when it failed
        #[qinvokable]
        #[cxx_name = "exportTo"]
        fn export_to(self: Pin<&mut Agenda>, calendar: &QString, path: &QString) -> bool;
        // The file or link Agenda was started with (a text/calendar file handed over by the system), empty otherwise
        #[qinvokable]
        #[cxx_name = "startupSource"]
        fn startup_source(self: &Agenda) -> QString;
        // Connects a CalDAV account (address, user, password) in a process of its own: the calendars of the account appear when
        // it is done, and `message` says how it went. The password goes to the keyring, nowhere else.
        #[qinvokable]
        #[cxx_name = "accountConnect"]
        fn account_connect(self: Pin<&mut Agenda>, address: &QString, user: &QString, password: &QString);
        // Fetches the subscriptions now (in the background); the files are read again a little later
        #[qinvokable]
        #[cxx_name = "syncNow"]
        fn sync_now(self: &Agenda);
        // Changes `name`, `color` or `hidden` ("true"/"false") of a calendar
        #[qinvokable]
        #[cxx_name = "calendarSet"]
        fn calendar_set(self: Pin<&mut Agenda>, slug: &QString, key: &QString, value: &QString) -> bool;
        // Deletes a calendar and its events (to the trash)
        #[qinvokable]
        #[cxx_name = "calendarRemove"]
        fn calendar_remove(self: Pin<&mut Agenda>, slug: &QString) -> bool;
        // The `[agenda]` settings of fili-cal.toml as JSON (see settings.rs)
        #[qinvokable]
        fn settings(self: &Agenda) -> QString;
        #[qinvokable]
        fn undo(self: Pin<&mut Agenda>) -> bool;
        #[qinvokable]
        fn redo(self: Pin<&mut Agenda>) -> bool;
        // {"undo": bool, "redo": bool, "label": "what an undo takes back"}
        #[qinvokable]
        #[cxx_name = "undoState"]
        fn undo_state(self: &Agenda) -> QString;
        // The one-line form ("tomorrow 14:30 Dentist", "+30m Bread") read as an event, as JSON, or {} when it is not one
        #[qinvokable]
        #[cxx_name = "parseQuick"]
        fn parse_quick(self: &Agenda, text: &QString) -> QString;
        // The date of today and the current minute (YYYY-MM-DDTHH:MM)
        #[qinvokable]
        fn now(self: &Agenda) -> QString;
        #[qinvokable]
        #[cxx_name = "clearMessage"]
        fn clear_message(self: Pin<&mut Agenda>);
        // Alarms due and file of the clock card (what the fili-bar runs every minute); the window does it too while open
        #[qinvokable]
        fn tick(self: Pin<&mut Agenda>);
    }

    impl cxx_qt::Threading for Fili {}
    impl cxx_qt::Initialize for Fili {}
    impl cxx_qt::Threading for Agenda {}
    impl cxx_qt::Initialize for Agenda {}
}

use std::{path::PathBuf, pin::Pin};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::{cli, i18n, model, theme, window};

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn state_dir() -> PathBuf {
    PathBuf::from(std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{}/.local/state", home()))).join("filigrane")
}

fn ui_file() -> PathBuf {
    state_dir().join("fili-cal-ui")
}

// `key=value` lines: reading of a key
fn ui_get(text: &str, key: &str) -> String {
    text.lines().find_map(|l| l.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v.trim().to_string())).unwrap_or_default()
}

// The same text with the key set (or removed when the value is empty)
fn ui_set(text: &str, key: &str, value: &str) -> String {
    let clean = |s: &str| s.replace(['\n', '\r'], " ");
    let (key, value) = (clean(key).replace('=', "-"), clean(value));
    let mut lines: Vec<String> =
        text.lines().filter(|l| l.split_once('=').is_none_or(|(k, _)| k != key)).map(String::from).collect();
    if !value.trim().is_empty() {
        lines.push(format!("{key}={}", value.trim()));
    }
    lines.sort();
    if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" }
}

fn ui_store(key: &str, value: &str) {
    let path = ui_file();
    let text = ui_set(&std::fs::read_to_string(&path).unwrap_or_default(), key, value);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, text);
}

// State of fili-theme ("light" or "dark"), like FiliTheme.qml of the fili-shell
fn read_light() -> bool {
    std::fs::read_to_string(state_dir().join("theme")).map(|t| t.trim() == "light").unwrap_or(false)
}

fn read_accent() -> String {
    std::fs::read_to_string(state_dir().join("accent")).map(|t| t.trim().to_string()).unwrap_or_else(|_| "default".into())
}

pub fn args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

/// Waits for changes in a directory (inotify, blocked on read: nothing wakes up while nothing changes) and calls `then`
/// until it returns false.
fn watch_dir(dir: PathBuf, then: impl Fn() -> bool + Send + 'static) {
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(&dir);
        let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) else { return };
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        if fd < 0
            || unsafe { libc::inotify_add_watch(fd, c.as_ptr(), libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_CREATE) } < 0
        {
            return;
        }
        let mut buf = [0u8; 4096];
        loop {
            if unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) } <= 0 || !then() {
                return;
            }
        }
    });
}

#[derive(Default)]
pub struct FiliRust {
    light: bool,
    accent: QString,
    settings_rev: i32,
    key_nav: bool,
}

impl cxx_qt::Initialize for qobject::Fili {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().light = read_light();
        self.as_mut().rust_mut().accent = QString::from(&read_accent());
        // The settings directory (fili-cal.toml, theme.toml written by `fili-theme font|header` or by hand)
        let qt = self.qt_thread();
        if let Some(dir) = theme::path().parent() {
            watch_dir(dir.to_path_buf(), move || {
                qt.queue(|mut f| {
                    let rev = f.settings_rev.wrapping_add(1);
                    f.as_mut().rust_mut().settings_rev = rev;
                    f.as_mut().theme_changed();
                })
                .is_ok()
            });
        }
        // The state directory (light or dark, accent)
        let qt = self.qt_thread();
        watch_dir(state_dir(), move || {
            let (light, accent) = (read_light(), read_accent());
            qt.queue(move |mut f| {
                if f.light != light || f.accent.to_string() != accent {
                    f.as_mut().rust_mut().light = light;
                    f.as_mut().rust_mut().accent = QString::from(&accent);
                    f.as_mut().theme_changed();
                }
            })
            .is_ok()
        });
        // Commands from `fili-cal ctl`
        let qt = self.qt_thread();
        crate::ctl::serve(move |id, verb, arg| {
            let _ = qt.queue(move |mut f| f.as_mut().command(id, &QString::from(&verb), &QString::from(&arg)));
        });
    }
}

impl qobject::Fili {
    pub fn reply(&self, id: i32, answer: &QString) {
        crate::ctl::reply(id, &answer.to_string());
    }

    pub fn theme_json(&self) -> QString {
        QString::from(&theme::theme_json())
    }

    pub fn tr(&self, text: &QString) -> QString {
        QString::from(i18n::tr(&text.to_string()))
    }

    pub fn window_control(&self) -> bool {
        window::available()
    }

    pub fn window_state(&self) -> QString {
        QString::from(&window::state())
    }

    // Header of the window (toolbar): false = headless (settings and --no-header)
    pub fn header(&self) -> bool {
        theme::window_part("header", &args())
    }

    // Header and footer take the accent colour while the app needs the user (settings, --attention / --no-attention)
    pub fn attention(&self) -> bool {
        theme::window_part("attention", &args())
    }

    // Footer (status bar): false = hidden until it has something to say (settings and --no-footer)
    pub fn footer(&self) -> bool {
        theme::window_part("footer", &args())
    }

    pub fn window_buttons(&self) -> bool {
        ui_get(&std::fs::read_to_string(ui_file()).unwrap_or_default(), "window_buttons") != "0"
    }

    pub fn set_window_buttons(&self, on: bool) {
        ui_store("window_buttons", if on { "1" } else { "0" });
    }

    pub fn window_action(&self, name: &QString) -> bool {
        window::run(&name.to_string())
    }

    pub fn ui_state(&self, key: &QString) -> QString {
        QString::from(&ui_get(&std::fs::read_to_string(ui_file()).unwrap_or_default(), &key.to_string()))
    }

    pub fn set_ui_state(&self, key: &QString, value: &QString) {
        ui_store(&key.to_string(), &value.to_string());
    }

    pub fn about_rows(&self) -> QString {
        let version =
            option_env!("FILI_CAL_GIT").map(str::to_string).unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        let rows = [
            (i18n::tr("Also known as"), "fili-cal".to_string()),
            (i18n::tr("Part of"), "Filigrane".to_string()),
            (i18n::tr("Version"), version),
            (i18n::tr("Events"), cli::store().display().to_string()),
            (i18n::tr("Format"), "iCalendar (.ics)".to_string()),
            (i18n::tr("Settings"), "~/.config/filigrane/fili-cal.toml".to_string()),
        ];
        let items: Vec<String> =
            rows.iter().map(|(k, v)| format!("[{},{}]", theme::json_str(k), theme::json_str(v))).collect();
        QString::from(&format!("[{}]", items.join(",")))
    }

    pub fn logs(&self, since: i32) -> QString {
        let (seq, entries) = crate::logs::since(since.max(0) as u64);
        QString::from(&serde_json::json!({ "seq": seq, "entries": entries }).to_string())
    }

    pub fn log_file(&self) -> QString {
        QString::from(&crate::logs::path().display().to_string())
    }

    pub fn clear_logs(&self) {
        crate::logs::clear();
    }

    pub fn open_path(&self, path: &QString) {
        let _ = std::process::Command::new("xdg-open")
            .arg(path.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    pub fn ask_lia(&self) {
        let child = std::process::Command::new("fili-ai")
            .arg("here")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        if let Ok(mut child) = child {
            std::thread::spawn(move || child.wait());
        }
    }

    pub fn lia_available(&self) -> bool {
        std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join("fili-ai-daemon").is_file()))
    }

    pub fn report_problem(&self, text: &QString) -> QString {
        let said = text.to_string();
        let message = if said.trim().is_empty() { "Problem reported from the Debug panel".to_string() } else { said };
        QString::from(&crate::logs::incident("manual", &message, true).unwrap_or_default())
    }

    pub fn open_settings(&self) {
        crate::settings::ensure_file();
        let _ = std::process::Command::new("xdg-open")
            .arg(theme::path())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    pub fn copy_text(&self, text: &QString) {
        use std::io::Write;
        if let Ok(mut c) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            && let Some(mut stdin) = c.stdin.take()
        {
            let _ = stdin.write_all(text.to_string().as_bytes());
        }
    }
}

pub struct AgendaRust {
    events: QString,
    message: QString,
    failed: bool,
    // The range shown: from, to (YYYY-MM-DD)
    range: (String, String),
    // What the footer says about unreadable files (so that it is cleared when they are fixed, and nothing else is)
    problem: String,
}

impl Default for AgendaRust {
    fn default() -> Self {
        AgendaRust { events: QString::from("[]"), message: QString::default(), failed: false, range: (String::new(), String::new()), problem: String::new() }
    }
}

impl cxx_qt::Initialize for qobject::Agenda {
    fn initialize(self: Pin<&mut Self>) {}
}

impl qobject::Agenda {
    fn fail(mut self: Pin<&mut Self>, text: String) {
        self.as_mut().rust_mut().message = QString::from(&text);
        self.as_mut().rust_mut().failed = true;
        self.as_mut().changed();
    }

    pub fn load(mut self: Pin<&mut Self>, from: &QString, to: &QString) {
        self.as_mut().rust_mut().range = (from.to_string(), to.to_string());
        self.refresh();
    }

    pub fn refresh(mut self: Pin<&mut Self>) {
        let (from, to) = self.range.clone();
        // how a connection that was started here ended
        let note = cli::state().join("fili-cal").join("connect.txt");
        if let Ok(text) = std::fs::read_to_string(&note) {
            let _ = std::fs::remove_file(&note);
            let (kind, rest) = text.split_once('\t').unwrap_or(("error", &text));
            let shown = if kind == "ok" { i18n::trf("Connected: %1", &[&rest]) } else { rest.to_string() };
            self.as_mut().rust_mut().message = QString::from(&shown);
            self.as_mut().rust_mut().failed = kind != "ok";
            self.as_mut().changed();
        }
        let scan = cli::scan();
        let json = model::events_json_of(&scan.events, &from, &to);
        // Files that cannot be read are told in the footer instead of vanishing
        let problem = match scan.unreadable.as_slice() {
            [] => String::new(),
            [(id, why)] => i18n::trf("The file %1.ics cannot be read (%2)", &[&id, &why]),
            list => i18n::trf("%1 files cannot be read, the first is %2.ics", &[&list.len(), &list[0].0]),
        };
        if problem != self.problem {
            let shown = self.message.to_string();
            self.as_mut().rust_mut().message = if problem.is_empty() && shown != self.problem { self.message.clone() } else { QString::from(&problem) };
            self.as_mut().rust_mut().problem = problem;
            self.as_mut().rust_mut().failed = false;
            self.as_mut().changed();
        }
        if self.events.to_string() != json {
            self.as_mut().rust_mut().events = QString::from(&json);
            self.as_mut().changed();
        }
    }

    pub fn add(mut self: Pin<&mut Self>, json: &QString) -> QString {
        match model::add(&json.to_string()) {
            Ok(id) => {
                self.as_mut().refresh();
                QString::from(&id)
            }
            Err(e) => {
                self.as_mut().fail(e);
                QString::default()
            }
        }
    }

    pub fn update(mut self: Pin<&mut Self>, id: &QString, json: &QString) -> bool {
        match model::update(&id.to_string(), &json.to_string()) {
            Ok(()) => {
                self.as_mut().refresh();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn remove(mut self: Pin<&mut Self>, id: &QString, occ: &QString) -> bool {
        match model::remove(&id.to_string(), &occ.to_string()) {
            Ok(()) => {
                self.as_mut().refresh();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn undo(mut self: Pin<&mut Self>) -> bool {
        self.step_back(false)
    }

    pub fn redo(mut self: Pin<&mut Self>) -> bool {
        self.step_back(true)
    }

    fn step_back(mut self: Pin<&mut Self>, again: bool) -> bool {
        match if again { crate::undo::redo() } else { crate::undo::undo() } {
            Ok(label) => {
                self.as_mut().refresh();
                let text = if again { i18n::trf("Done again: %1", &[&label]) } else { i18n::trf("Undone: %1", &[&label]) };
                self.as_mut().rust_mut().message = QString::from(&text);
                self.as_mut().rust_mut().failed = false;
                self.as_mut().changed();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn calendars(&self) -> QString {
        QString::from(&crate::calendars::json())
    }

    pub fn preview_import(&self, source: &QString) -> QString {
        QString::from(&crate::exchange::preview(&source.to_string()).to_string())
    }

    pub fn import_source(mut self: Pin<&mut Self>, source: &QString, calendar: &QString) -> bool {
        match crate::exchange::import(&source.to_string(), &calendar.to_string()) {
            Ok((added, replaced)) => {
                self.as_mut().refresh();
                let text = if replaced == 0 { i18n::trf("Imported %1 events", &[&added]) } else { i18n::trf("Imported %1 events, %2 replaced", &[&(added + replaced), &replaced]) };
                self.as_mut().rust_mut().message = QString::from(&text);
                self.as_mut().rust_mut().failed = false;
                self.as_mut().changed();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn export_to(mut self: Pin<&mut Self>, calendar: &QString, path: &QString) -> bool {
        let cal = calendar.to_string();
        match crate::exchange::export_to(Some(&cal), &path.to_string()) {
            Ok(file) => {
                self.as_mut().rust_mut().message = QString::from(&i18n::trf("Exported to %1", &[&file]));
                self.as_mut().rust_mut().failed = false;
                self.as_mut().changed();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn startup_source(&self) -> QString {
        QString::from(&std::env::var("FILI_CAL_OPEN").unwrap_or_default())
    }

    pub fn account_connect(mut self: Pin<&mut Self>, address: &QString, user: &QString, password: &QString) {
        let (address, user, password) = (address.to_string(), user.to_string(), password.to_string());
        let Ok(exe) = std::env::current_exe() else { return };
        let child = std::process::Command::new(exe).args(["calendar", "connect", &address, &user]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn();
        let Ok(mut child) = child else {
            self.as_mut().fail(i18n::tr("could not start the connection").to_string());
            return;
        };
        if let Some(mut pipe) = child.stdin.take() {
            use std::io::Write;
            let _ = writeln!(pipe, "{password}");
        }
        self.as_mut().rust_mut().message = QString::from(&i18n::tr("Connecting…").to_string());
        self.as_mut().rust_mut().failed = false;
        self.as_mut().changed();
        // the result is told by a file, read by the next refresh
        std::thread::spawn(move || {
            let note = match child.wait_with_output() {
                Ok(o) if o.status.success() => format!("ok\t{}", String::from_utf8_lossy(&o.stdout).lines().collect::<Vec<_>>().join(", ")),
                Ok(o) => format!("error\t{}", String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("").trim_start_matches("fili-cal: ")),
                Err(e) => format!("error\t{e}"),
            };
            let dir = cli::state().join("fili-cal");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("connect.txt"), note);
        });
    }

    pub fn sync_now(&self) {
        // every subscription and account, due or not: a process of its own that waits for none
        if let Ok(exe) = std::env::current_exe() {
            for c in crate::calendars::list().iter().filter(|c| c.url.is_some() || c.caldav.is_some()) {
                let _ = std::process::Command::new(&exe).args(["calendar", "sync", &c.slug]).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
            }
        }
    }

    pub fn calendar_add(mut self: Pin<&mut Self>, name: &QString, color: &QString, url: &QString) -> QString {
        let url = url.to_string();
        let made = if url.trim().is_empty() { crate::calendars::create(&name.to_string(), &color.to_string()) } else { crate::calendars::subscribe(&name.to_string(), &url, &color.to_string(), None) };
        if made.is_ok() && !url.trim().is_empty() {
            self.sync_now();
        }
        match made {
            Ok(c) => {
                self.as_mut().refresh();
                QString::from(&c.slug)
            }
            Err(e) => {
                self.as_mut().fail(e);
                QString::default()
            }
        }
    }

    pub fn calendar_set(mut self: Pin<&mut Self>, slug: &QString, key: &QString, value: &QString) -> bool {
        self.calendar_done(crate::calendars::update(&slug.to_string(), &key.to_string(), &value.to_string()))
    }

    pub fn calendar_remove(mut self: Pin<&mut Self>, slug: &QString) -> bool {
        let slug = slug.to_string();
        crate::dav::forget_password(&slug);
        self.calendar_done(crate::calendars::remove(&slug))
    }

    fn calendar_done(mut self: Pin<&mut Self>, result: Result<(), String>) -> bool {
        match result {
            Ok(()) => {
                // the events of the views depend on which calendars are shown: reload even when the files did not change
                self.as_mut().rust_mut().events = QString::from("[]");
                self.as_mut().refresh();
                self.as_mut().changed();
                true
            }
            Err(e) => {
                self.as_mut().fail(e);
                false
            }
        }
    }

    pub fn settings(&self) -> QString {
        QString::from(&crate::settings::json())
    }

    pub fn undo_state(&self) -> QString {
        let (undo, redo, label) = crate::undo::state();
        QString::from(&serde_json::json!({"undo": undo, "redo": redo, "label": label}).to_string())
    }

    pub fn parse_quick(&self, text: &QString) -> QString {
        QString::from(&model::parse_quick(&text.to_string()))
    }

    pub fn now(&self) -> QString {
        QString::from(&crate::when::now().format("%Y-%m-%dT%H:%M").to_string())
    }

    pub fn clear_message(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().message = QString::default();
        self.as_mut().rust_mut().failed = false;
        self.as_mut().changed();
    }

    pub fn tick(mut self: Pin<&mut Self>) {
        let _ = cli::tick();
        self.as_mut().refresh();
    }
}
