// Window of Agenda (fili-cal): the calendar and the reminders of Filigrane. Header (previous / next / today, the period, day · week ·
// month, new event, window buttons), sidebar (the small month, what comes next), the view (month grid, or the hours of a day or a
// week), a footer: the anatomy of every fili-* window (app-window.md of the filigrane-ui skill, Casier is the reference).
// A reminder is an event with an alert at its start, so there is one list. Shortcuts: Ctrl+N new event, Ctrl+Shift+N quick add,
// Ctrl+T today, Ctrl+1 / 2 / 3 day · week · month, Left / Right previous / next period, Ctrl+W close.
import QtQuick
import QtQuick.Controls.Basic
import Fili.Agenda
import Fili.UI

ApplicationWindow {
    id: win
    // The kit reads the theme through the application's object. Evaluated first: the tokens must be right from the first binding
    readonly property bool kitReady: { FiliHost.backend = Fili; return true; }
    // Fonts embedded in the binary: same look on every distribution
    FontLoader { id: fontText; source: "qrc:/qt/qml/Fili/Agenda/fonts/Jost-Variable.ttf" }
    FontLoader { id: fontMono; source: "qrc:/qt/qml/Fili/Agenda/fonts/JetBrainsMono-Variable.ttf" }
    FontLoader { id: fontIcons; source: "qrc:/qt/qml/Fili/Agenda/fonts/FiliIcons.ttf" }
    width: 1120
    height: 720
    minimumWidth: 760
    minimumHeight: 480
    visible: true
    color: FiliTheme.background
    title: "Agenda"

    Agenda { id: cal }
    // What comes next (sidebar): a second object, its own range
    Agenda { id: soon }
    DateTools { id: tools; prefs: JSON.parse(cal.settings()); accents: JSON.parse(Fili.themeJson()).accents; light: Fili.light }

    // ---- State ---------------------------------------------------------------------------------------------------
    // `--view day|week|month` overrides the view kept from the last launch
    readonly property var launchArgs: Qt.application.arguments
    readonly property var viewNames: ["day", "week", "month", "year", "list"]
    property string viewMode: launchArgs.includes("--view") && viewNames.includes(launchArgs[launchArgs.indexOf("--view") + 1]) ? launchArgs[launchArgs.indexOf("--view") + 1]
        : viewNames.includes(Fili.uiState("view")) ? Fili.uiState("view") : "week"
    property date today: tools.day(new Date())
    property date now: new Date()
    property date anchor: tools.day(new Date())
    readonly property int firstDay: tools.prefs.firstDay >= 0 ? tools.prefs.firstDay : Qt.locale().firstDayOfWeek
    property bool aboutOpen: false
    property bool showHeader: Fili.header()
    property bool showFooter: Fili.footer()
    property bool windowButtons: Fili.windowButtons()
    // The app waits for the user (attentionText says why): header and footer take the accent colour when the setting is on
    property bool attentionOn: Fili.attention()
    property string attentionText: ""
    readonly property bool attending: attentionOn && attentionText !== ""
    readonly property color onAccent: (0.299 * FiliTheme.accentColor.r + 0.587 * FiliTheme.accentColor.g + 0.114 * FiliTheme.accentColor.b) > 0.55 ? "#14181f" : "#ffffff"
    readonly property color tintSurface: FiliTheme.alpha(onAccent, 0.12)
    readonly property color tintHover: FiliTheme.alpha(onAccent, 0.2)
    readonly property color tintBorder: FiliTheme.alpha(onAccent, 0.3)
    function attend(text) { attentionText = text || Fili.tr("Needs your attention"); }
    onActiveChanged: if (active) attentionText = ""
    readonly property bool headerShown: showHeader
    readonly property bool footerShown: showFooter || cal.message !== "" || attending
    // A dialog or the quick add is open: the keys of the window leave it alone
    readonly property bool typing: quick.visible || editor.visible || importDialog.visible || exportDialog.visible || searchDialog.visible || accountDialog.visible || calendarDialog.visible
    // The event the user last pointed at (a click, a right click): the one Copy, Duplicate, Delete and Enter act on; and the one copied
    property var selected: null
    property var clip: null
    readonly property string selectedKey: selected ? selected.id + "|" + selected.occ : ""
    function select(ev) { selected = ev; Qt.callLater(publishContext); }
    function eventText(ev) {
        const s = tools.start(ev);
        return ev.title + " — " + Qt.locale().toString(s, ev.allDay ? "dddd d MMMM yyyy" : "dddd d MMMM yyyy, HH:mm") + (ev.place ? " · " + ev.place : "");
    }
    // An event as the JSON that makes a new one starting at `at` (the same length, the same alerts)
    function copyJson(ev, at) {
        const text = d => ev.allDay ? tools.iso(d) : tools.stamp(d);
        const s = tools.start(ev), len = ev.end ? tools.parse(ev.end) - s : 0;
        return JSON.stringify({ title: ev.title, allDay: ev.allDay, start: text(at), end: ev.end ? text(new Date(at.getTime() + len)) : "", place: ev.place, note: ev.note, alarms: ev.alarms, calendar: ev.readonly ? "" : ev.calendar });
    }
    function duplicateEvent(ev) { cal.add(copyJson(ev, tools.start(ev))); soon.refresh(); }
    function copyEvent(ev) { clip = ev; Fili.copyText(eventText(ev)); }
    function pasteAt(day) {
        if (!clip)
            return;
        const s = tools.start(clip);
        cal.add(copyJson(clip, new Date(day.getFullYear(), day.getMonth(), day.getDate(), s.getHours(), s.getMinutes())));
        soon.refresh();
    }
    function toggleDone(ev) { cal.update(ev.id, JSON.stringify({ done: !ev.done })); soon.refresh(); }
    // The next or the previous event of the period in reading order (Tab, Shift+Tab); the day follows it
    function selectStep(n) {
        const list = events.filter(e => true);
        if (list.length === 0)
            return;
        const at = selected ? list.findIndex(e => e.id === selected.id && e.occ === selected.occ) : -1;
        const to = list[Math.max(0, Math.min(list.length - 1, at < 0 ? (n > 0 ? 0 : list.length - 1) : at + n))];
        select(to);
        if (!tools.sameDay(tools.start(to), anchor) && viewMode !== "week" && viewMode !== "list")
            anchor = tools.day(tools.start(to));
    }
    // Moves the selected event by minutes (its end by `dEnd`); the selection follows it
    function moveSelected(dStart, dEnd) {
        const ev = selected;
        if (!ev || ev.readonly)
            return;
        const shifted = new Date(tools.parse(ev.occ));
        shifted.setMinutes(shifted.getMinutes() + dStart);
        changeEvent(ev, dStart, dEnd, false);
        selected = Object.assign({}, ev, { occ: ev.allDay ? tools.iso(shifted) : tools.stamp(shifted), start: ev.allDay ? tools.iso(shifted) : tools.stamp(shifted) });
    }
    function deleteEvent(ev, all) {
        applyDeleted(ev.id, ev.occ, all);
        if (selected && selected.id === ev.id)
            selected = null;
    }
    // Moves the day the user is on (the selected day of the month, the day of the week)
    function moveDay(n) { anchor = tools.addDays(anchor, n); }
    // The menu of a right click
    property var ctxEvent: null
    property date ctxDay: new Date()
    function showContext(ev, x, y) {
        select(ev);
        ctxEvent = ev;
        const o = ctxMenu.parent.mapToItem(null, 0, 0);
        ctxMenu.open(x - o.x, y - o.y, [
            { id: "edit", label: ev.readonly ? Fili.tr("Open") : Fili.tr("Edit…") },
            { id: "duplicate", label: Fili.tr("Duplicate") },
            { id: "copy", label: Fili.tr("Copy"), hint: "Ctrl+C" },
            ...(ev.allDay && !ev.readonly ? [{ id: "done", label: ev.done ? Fili.tr("Mark as not done") : Fili.tr("Mark as done") }] : []),
            { separator: true },
            { id: "delete", label: ev.series ? Fili.tr("Delete this occurrence") : Fili.tr("Delete"), enabled: !ev.readonly, danger: true, hint: "Del" },
            ...(ev.series && !ev.readonly ? [{ id: "delete-series", label: Fili.tr("Delete the series"), danger: true }] : [])
        ]);
    }
    function showSlotContext(day, x, y) {
        ctxEvent = null;
        ctxDay = day;
        const o = ctxMenu.parent.mapToItem(null, 0, 0);
        ctxMenu.open(x - o.x, y - o.y, [
            { id: "new", label: Fili.tr("New event here…") },
            { id: "paste", label: Fili.tr("Paste"), enabled: clip !== null, hint: "Ctrl+V" }
        ]);
    }
    readonly property var events: JSON.parse(cal.events)
    // The calendars [{slug, name, color, hidden}]
    property var calendars: JSON.parse(cal.calendars())
    function reloadCalendars() { calendars = JSON.parse(cal.calendars()); }
    function setCalendar(slug, key, value) { if (cal.calendarSet(slug, key, value)) { reloadCalendars(); soon.refresh(); } }
    function saveCalendar(slug, name, color, url) {
        if (slug === "") {
            const s = cal.calendarAdd(name, color, url);
            if (s === "")
                return;
        } else {
            cal.calendarSet(slug, "name", name);
            cal.calendarSet(slug, "color", color);
        }
        reloadCalendars();
        soon.refresh();
    }
    readonly property var upcoming: JSON.parse(soon.events).filter(e => tools.end(e) >= win.now || e.allDay).slice(0, 12)

    // The range the view shows: [from, to)
    readonly property date rangeStart: viewMode === "month" ? tools.weekStart(new Date(anchor.getFullYear(), anchor.getMonth(), 1), firstDay) : viewMode === "week" ? tools.weekStart(anchor, firstDay)
        : viewMode === "year" ? new Date(anchor.getFullYear(), 0, 1) : anchor
    // The list shows the next 60 days; the year, its twelve months
    readonly property date rangeEnd: viewMode === "year" ? new Date(anchor.getFullYear() + 1, 0, 1) : tools.addDays(rangeStart, viewMode === "month" ? 42 : viewMode === "week" ? 7 : viewMode === "list" ? 60 : 1)
    // Not onRangeStartChanged: the first evaluation of a binding would reload in the middle of another binding (a loop)
    onAnchorChanged: { Qt.callLater(reload); Qt.callLater(publishContext); }
    onViewModeChanged: { Qt.callLater(reload); Qt.callLater(publishContext); }
    onEventsChanged: Qt.callLater(publishContext)
    Component.onDestruction: cal.clearContext()
    // What the window shows, for the assistant and `fili-context` (no places or notes beyond the title, place and times)
    function publishContext() {
        const ev = e => ({ id: e.id, title: e.title, start: e.start, end: e.end, all_day: e.allDay, place: e.place, calendar: e.calendar });
        const sel = editor.visible && editor.eventId !== "" ? events.find(e => e.id === editor.eventId && e.occ === editor.occ) : selected ? events.find(e => e.id === selected.id && e.occ === selected.occ) : undefined;
        cal.publishContext(JSON.stringify({ view: viewMode, from: tools.iso(rangeStart), to: tools.iso(rangeEnd), day: tools.iso(anchor), events: events.slice(0, 60).map(ev), selected: sel ? ev(sel) : null }));
    }
    function reload() { cal.load(tools.iso(rangeStart), tools.iso(rangeEnd)); }
    function reloadSoon() { soon.load(tools.iso(today), tools.iso(tools.addDays(today, 14))); }
    Component.onCompleted: {
        reload();
        reloadSoon();
        // handed a file or a link by the system (text/calendar, webcal:)
        if (cal.startupGoto() !== "")
            anchor = tools.day(tools.parse(cal.startupGoto()));
        if (cal.startupSource() !== "")
            Qt.callLater(() => importDialog.show(cal.startupSource()));
    }
    // A file dropped on the window: its import preview
    DropArea {
        anchors.fill: parent
        z: 1000
        keys: ["text/uri-list"]
        onDropped: drop => {
            if (!drop.hasUrls || drop.urls.length === 0)
                return;
            // on an open event the file is an attachment, elsewhere an iCalendar file to import
            if (editor.visible && !editor.readOnly)
                drop.urls.forEach(u => editor.attach(u.toString()));
            else if (!win.typing)
                importDialog.show(drop.urls[0].toString());
        }
    }
    // `fili-cal --shot FILE [view]`: draws the window into a PNG and quits (documentation, tests without a screen)
    Timer {
        readonly property var args: Qt.application.arguments
        readonly property int at: args.indexOf("--shot")
        interval: 1500
        running: at >= 0 && at + 1 < args.length
        onTriggered: scene.grabToImage(r => { r.saveToFile(args[at + 1]); Qt.quit(); })
    }

    readonly property string periodTitle: {
        const L = Qt.locale();
        if (viewMode === "month")
            return L.monthName(anchor.getMonth(), Locale.LongFormat) + " " + anchor.getFullYear();
        if (viewMode === "year")
            return String(anchor.getFullYear());
        if (viewMode === "list")
            return Fili.tr("From %1").replace("%1", anchor.getDate() + " " + L.monthName(anchor.getMonth(), Locale.LongFormat) + " " + anchor.getFullYear());
        if (viewMode === "day")
            return L.dayName(anchor.getDay(), Locale.LongFormat) + " " + anchor.getDate() + " " + L.monthName(anchor.getMonth(), Locale.LongFormat) + " " + anchor.getFullYear();
        const a = rangeStart, b = tools.addDays(rangeStart, 6);
        const wk = tools.prefs.weekNumbers === true ? Fili.tr("Week %1").replace("%1", tools.isoWeek(tools.addDays(a, (4 - firstDay + 7) % 7))) + " · " : "";
        return wk + (a.getMonth() === b.getMonth() ? a.getDate() + " – " + b.getDate() + " " + L.monthName(b.getMonth(), Locale.LongFormat) + " " + b.getFullYear()
            : a.getDate() + " " + L.monthName(a.getMonth(), Locale.ShortFormat) + " – " + b.getDate() + " " + L.monthName(b.getMonth(), Locale.ShortFormat) + " " + b.getFullYear());
    }
    function step(n) {
        anchor = viewMode === "year" ? tools.addMonths(anchor, 12 * n) : viewMode === "month" ? tools.addMonths(anchor, n) : tools.addDays(anchor, n * (viewMode === "week" ? 7 : viewMode === "list" ? 14 : 1));
    }
    function goToday() { anchor = tools.day(new Date()); }
    function setView(m) {
        viewMode = m;
        Fili.setUiState("view", m);
    }
    function quit() { Qt.quit(); }
    function whenText(ev) {
        const s = tools.start(ev), L = Qt.locale();
        const day = tools.sameDay(s, today) ? Fili.tr("Today") : tools.sameDay(s, tools.addDays(today, 1)) ? Fili.tr("Tomorrow") : L.dayName(s.getDay(), Locale.ShortFormat) + " " + s.getDate();
        return ev.allDay ? day : day + "  " + tools.clock(ev);
    }
    function newEvent(at) { editor.create(at ?? (["month", "year", "list"].includes(viewMode) ? anchor : new Date(anchor.getFullYear(), anchor.getMonth(), anchor.getDate(), Math.min(23, Math.max(9, now.getHours() + 1)), 0))); }
    // A block moved or resized in the week: its start and end move by `dStart` and `dEnd` minutes. A series moves from the
    // occurrence that was dragged (its first start moves by as much); Alt makes a copy that is a single event.
    function changeEvent(ev, dStart, dEnd, copy) {
        const shift = (d, n) => { const r = new Date(d); r.setMinutes(r.getMinutes() + n); return r; };
        // a whole day is written as a date, a timed event as date and time
        const text = d => ev.allDay ? tools.iso(d) : tools.stamp(d);
        const occ = tools.start(ev), newStart = shift(occ, dStart);
        const newEnd = ev.end ? shift(tools.parse(ev.end), dEnd) : null;
        if (copy) {
            cal.add(JSON.stringify({ title: ev.title, allDay: ev.allDay, start: text(newStart), end: newEnd ? text(newEnd) : "", place: ev.place, note: ev.note, alarms: ev.alarms, calendar: ev.calendar }));
            soon.refresh();
            return;
        }
        const first = ev.series ? shift(tools.parse(ev.first), dStart) : newStart;
        const o = { start: text(first) };
        if (newEnd)
            o.end = text(shift(first, Math.round((newEnd - newStart) / 60000)));
        cal.update(ev.id, JSON.stringify(o));
        soon.refresh();
    }
    function applySaved(id, json) {
        if (id === "")
            cal.add(json);
        else
            cal.update(id, json);
        soon.refresh();
    }
    function applyDeleted(id, occ, all) {
        cal.remove(id, all ? "" : (occ !== "" && cal.events.indexOf('"series":true') >= 0 && win.events.some(e => e.id === id && e.series) ? occ : ""));
        soon.refresh();
    }
    onClosing: close => {}

    Connections {
        target: Fili
        function onThemeChanged() {
            win.showHeader = Fili.header();
            win.showFooter = Fili.footer();
            win.attentionOn = Fili.attention();
            tools.accents = JSON.parse(Fili.themeJson()).accents;
            tools.light = Fili.light;
        }
    }
    // What makes Agenda ask for the user when the window is not the active one: an event that could not be saved
    Connections {
        target: cal
        function onChanged() {
            if (cal.failed && cal.message !== "" && !win.active)
                win.attend(cal.message);
        }
    }
    // The clock: the red line, today, and the alerts that are due (the fili-bar does it too, a second time is harmless: an
    // alert goes off once). A look at the files meanwhile: an event added from the command line or by Lia shows up.
    Timer {
        interval: 30000
        repeat: true
        running: win.visible
        onTriggered: {
            win.now = new Date();
            const d = tools.day(win.now);
            if (!tools.sameDay(d, win.today)) {
                const follow = tools.sameDay(win.anchor, win.today);
                win.today = d;
                if (follow)
                    win.anchor = d;
                win.reloadSoon();
            }
        }
    }
    Timer { interval: 60000; repeat: true; running: win.visible; onTriggered: cal.tick() }
    Timer { interval: 3000; repeat: true; running: win.visible; onTriggered: { cal.refresh(); soon.refresh(); tools.prefs = JSON.parse(cal.settings()); win.reloadCalendars(); } }
    Timer { interval: 6000; running: cal.message !== "" && !cal.failed; onTriggered: cal.clearMessage() }

    // ---- Commands: the menu of the fili-bar and the assistant (fili-cal ctl) -------------------------------------------
    readonly property var actions: ({
        "about": { on: () => true, run: () => win.aboutOpen = !win.aboutOpen },
        "view-day": { on: () => true, run: () => win.setView("day") },
        "view-week": { on: () => true, run: () => win.setView("week") },
        "view-month": { on: () => true, run: () => win.setView("month") },
        "view-year": { on: () => true, run: () => win.setView("year") },
        "view-list": { on: () => true, run: () => win.setView("list") },
        "today": { on: () => true, run: () => win.goToday() },
        "previous": { on: () => true, run: () => win.step(-1) },
        "next": { on: () => true, run: () => win.step(1) },
        "new-event": { on: () => !win.typing, run: () => win.newEvent() },
        "undo": { on: () => !win.typing && JSON.parse(cal.undoState()).undo, run: () => { cal.undo(); soon.refresh(); } },
        "redo": { on: () => !win.typing && JSON.parse(cal.undoState()).redo, run: () => { cal.redo(); soon.refresh(); } },
        "refresh-calendars": { on: () => win.calendars.some(c => c.subscribed || c.account), run: () => { cal.syncNow(); } },
        "next-event": { on: () => !win.typing, run: () => win.selectStep(1) },
        "previous-event": { on: () => !win.typing, run: () => win.selectStep(-1) },
        "move-earlier": { on: () => !win.typing && win.selected !== null && !win.selected.allDay, run: () => win.moveSelected(-15, -15) },
        "move-later": { on: () => !win.typing && win.selected !== null && !win.selected.allDay, run: () => win.moveSelected(15, 15) },
        "move-day-before": { on: () => !win.typing && win.selected !== null, run: () => win.moveSelected(-1440, -1440) },
        "move-day-after": { on: () => !win.typing && win.selected !== null, run: () => win.moveSelected(1440, 1440) },
        "shorten": { on: () => !win.typing && win.selected !== null && !win.selected.allDay && win.selected.end !== null, run: () => win.moveSelected(0, -15) },
        "lengthen": { on: () => !win.typing && win.selected !== null && !win.selected.allDay && win.selected.end !== null, run: () => win.moveSelected(0, 15) },
        "copy-event": { on: () => !win.typing && win.selected !== null, run: () => win.copyEvent(win.selected) },
        "paste-event": { on: () => !win.typing && win.clip !== null, run: () => win.pasteAt(win.anchor) },
        "duplicate-event": { on: () => !win.typing && win.selected !== null, run: () => win.duplicateEvent(win.selected) },
        "toggle-done": { on: () => !win.typing && win.selected !== null && win.selected.allDay && !win.selected.readonly, run: () => win.toggleDone(win.selected) },
        "delete-event": { on: () => !win.typing && win.selected !== null && !win.selected.readonly, run: () => win.deleteEvent(win.selected, false) },
        "search": { on: () => true, run: rest => searchDialog.show(rest) },
        "select": { on: () => true, run: rest => { const ev = win.events.find(e => e.id === rest.trim()); if (ev) editor.edit(ev); } },
        "goto": { on: () => true, run: rest => { const d = tools.parse(rest); if (!isNaN(d.getTime())) win.anchor = tools.day(d); } },
        "import": { on: () => !win.typing, run: () => importDialog.show("") },
        "import-file": { on: () => true, run: rest => { importDialog.show(rest); win.raise(); } },
        "export": { on: () => !win.typing, run: () => exportDialog.start() },
        "add-account": { on: () => !win.typing, run: () => accountDialog.start() },
        "new-calendar": { on: () => !win.typing, run: () => calendarDialog.create() },
        "quick-add": { on: () => !win.typing, run: () => quick.start() },
        "settings": { on: () => true, run: () => Fili.openSettings() },
        "toggle-header": { on: () => true, run: () => win.showHeader = !win.showHeader },
        "toggle-footer": { on: () => true, run: () => win.showFooter = !win.showFooter },
        "attention": { on: () => true, run: () => win.attend("") },
        "attention-clear": { on: () => win.attentionText !== "", run: () => win.attentionText = "" },
        "debug": { on: () => true, run: () => debug.open = !debug.open },
        "toggle-window-buttons": { on: () => true, run: () => { win.windowButtons = !win.windowButtons; Fili.setWindowButtons(win.windowButtons); } },
        "window-actions": { on: () => win.windowButtons, run: () => windowPanel.visible ? windowPanel.close() : windowPanel.open() },
        "close": { on: () => true, run: () => win.quit() }
    })
    function runAction(name, rest) {
        const a = actions[name];
        if (!a)
            return { ok: false, error: "unknown action: " + name + " (fili-cal ctl actions lists them)" };
        if (!a.on())
            return { ok: false, error: "action disabled now: " + name };
        a.run(rest);
        return { ok: true };
    }
    function stateJson() {
        return { view: viewMode, date: tools.iso(anchor), from: tools.iso(rangeStart), to: tools.iso(rangeEnd), events: events.length, title: periodTitle, editing: editor.visible, importing: importDialog.visible, searching: searchDialog.visible, selected: selected ? selected.id : "", calendars: calendars.length };
    }
    Connections {
        target: Fili
        function onCommand(id, verb, arg) {
            let answer;
            if (verb === "action")
                answer = win.runAction(arg.split(" ")[0], arg.split(" ").slice(1).join(" "));
            else if (verb === "actions")
                answer = { ok: true, actions: Object.keys(win.actions).map(n => ({ name: n, enabled: win.actions[n].on() })) };
            else if (verb === "state")
                answer = Object.assign({ ok: true }, win.stateJson());
            else
                answer = { ok: false, error: "unknown command: " + verb };
            Fili.reply(id, JSON.stringify(answer));
        }
    }

    // ---- Shortcuts -----------------------------------------------------------------------------------------------
    Shortcut { sequence: "Ctrl+W"; onActivated: win.quit() }
    Shortcut { sequence: "Ctrl+Q"; onActivated: win.quit() }
    Shortcut { sequence: "Ctrl+N"; enabled: !win.typing; onActivated: win.runAction("new-event") }
    Shortcut { sequence: "Ctrl+Alt+A"; enabled: !win.typing; onActivated: win.runAction("add-account") }
    Shortcut { sequence: "Tab"; enabled: !win.typing; onActivated: win.runAction("next-event") }
    Shortcut { sequence: "Shift+Tab"; enabled: !win.typing; onActivated: win.runAction("previous-event") }
    Shortcut { sequence: "Alt+Up"; enabled: !win.typing; onActivated: win.runAction("move-earlier") }
    Shortcut { sequence: "Alt+Down"; enabled: !win.typing; onActivated: win.runAction("move-later") }
    Shortcut { sequence: "Alt+Left"; enabled: !win.typing; onActivated: win.runAction("move-day-before") }
    Shortcut { sequence: "Alt+Right"; enabled: !win.typing; onActivated: win.runAction("move-day-after") }
    Shortcut { sequence: "Alt+Shift+Up"; enabled: !win.typing; onActivated: win.runAction("shorten") }
    Shortcut { sequence: "Alt+Shift+Down"; enabled: !win.typing; onActivated: win.runAction("lengthen") }
    Shortcut { sequence: "Ctrl+C"; enabled: !win.typing; onActivated: win.runAction("copy-event") }
    Shortcut { sequence: "Ctrl+V"; enabled: !win.typing; onActivated: win.runAction("paste-event") }
    Shortcut { sequence: "Ctrl+D"; enabled: !win.typing; onActivated: win.runAction("duplicate-event") }
    Shortcut { sequence: "Ctrl+Shift+D"; enabled: !win.typing; onActivated: win.runAction("toggle-done") }
    Shortcut { sequence: "Delete"; enabled: !win.typing; onActivated: win.runAction("delete-event") }
    // Plain keys: T today, 1 to 5 the views, arrows the day (Shift: a day in any view, Up and Down: a week), Enter opens the selected event or makes one
    Shortcut { sequence: "T"; enabled: !win.typing; onActivated: win.goToday() }
    Shortcut { sequence: "1"; enabled: !win.typing; onActivated: win.setView("day") }
    Shortcut { sequence: "2"; enabled: !win.typing; onActivated: win.setView("week") }
    Shortcut { sequence: "3"; enabled: !win.typing; onActivated: win.setView("month") }
    Shortcut { sequence: "4"; enabled: !win.typing; onActivated: win.setView("year") }
    Shortcut { sequence: "5"; enabled: !win.typing; onActivated: win.setView("list") }
    Shortcut { sequence: "Shift+Left"; enabled: !win.typing; onActivated: win.moveDay(-1) }
    Shortcut { sequence: "Shift+Right"; enabled: !win.typing; onActivated: win.moveDay(1) }
    Shortcut { sequence: "Up"; enabled: !win.typing && win.viewMode === "month"; onActivated: win.moveDay(-7) }
    Shortcut { sequence: "Down"; enabled: !win.typing && win.viewMode === "month"; onActivated: win.moveDay(7) }
    Shortcut { sequence: "Return"; enabled: !win.typing; onActivated: win.selected ? editor.edit(win.selected) : win.newEvent() }
    Shortcut { sequence: "Ctrl+F"; onActivated: win.runAction("search") }
    Shortcut { sequence: "Ctrl+I"; enabled: !win.typing; onActivated: win.runAction("import") }
    Shortcut { sequence: "Ctrl+E"; enabled: !win.typing; onActivated: win.runAction("export") }
    Shortcut { sequence: "Ctrl+R"; enabled: !win.typing; onActivated: win.runAction("refresh-calendars") }
    Shortcut { sequence: "Ctrl+Alt+N"; enabled: !win.typing; onActivated: win.runAction("new-calendar") }
    Shortcut { sequence: "Ctrl+Shift+N"; enabled: !win.typing; onActivated: win.runAction("quick-add") }
    Shortcut { sequence: "Ctrl+Z"; enabled: !win.typing; onActivated: win.runAction("undo") }
    Shortcut { sequence: "Ctrl+Shift+Z"; enabled: !win.typing; onActivated: win.runAction("redo") }
    Shortcut { sequence: "Ctrl+T"; onActivated: win.goToday() }
    Shortcut { sequence: "Ctrl+1"; onActivated: win.setView("day") }
    Shortcut { sequence: "Ctrl+2"; onActivated: win.setView("week") }
    Shortcut { sequence: "Ctrl+3"; onActivated: win.setView("month") }
    Shortcut { sequence: "Ctrl+4"; onActivated: win.setView("year") }
    Shortcut { sequence: "Ctrl+5"; onActivated: win.setView("list") }
    Shortcut { sequence: "Left"; enabled: !win.typing && !win.aboutOpen; onActivated: win.step(-1) }
    Shortcut { sequence: "Right"; enabled: !win.typing && !win.aboutOpen; onActivated: win.step(1) }
    Shortcut { sequence: "Ctrl+Shift+L"; onActivated: win.runAction("debug") }
    Shortcut { sequence: "Ctrl+Shift+W"; enabled: win.windowButtons; onActivated: windowPanel.visible ? windowPanel.close() : windowPanel.open() }
    Shortcut { sequence: "Escape"; enabled: win.aboutOpen; onActivated: win.aboutOpen = false }

    // Everything that is drawn sits in one item, so that a picture of the window can be taken (--shot)
    Item {
        id: scene
        anchors.fill: parent
        Rectangle { anchors.fill: parent; color: FiliTheme.background; z: -1 }

        // ---- Header ---------------------------------------------------------------------------------------------------
        Rectangle {
            id: toolbar
            width: parent.width
            height: win.headerShown ? FiliTheme.toolbarHeight : 0
            visible: win.headerShown
            color: win.attending ? FiliTheme.accentColor : FiliTheme.background
            MouseArea { anchors.fill: parent; z: -1; enabled: win.attending; onClicked: win.attentionText = "" }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: FiliTheme.borderWidth; color: FiliTheme.border }
            FiliNavCapsule {
                id: nav
                x: FiliTheme.margin
                anchors.verticalCenter: parent.verticalCenter
                canBack: true
                canForward: true
                onBack: win.step(-1)
                onForward: win.step(1)
            }
            FiliButton {
                id: todayButton
                anchors { left: nav.right; leftMargin: FiliTheme.space2; verticalCenter: parent.verticalCenter }
                label: Fili.tr("Today")
                onClicked: win.goToday()
            }
            Text {
                id: periodText
                anchors { left: todayButton.right; leftMargin: FiliTheme.space4; right: views.left; rightMargin: FiliTheme.space3; verticalCenter: parent.verticalCenter }
                text: win.periodTitle
                elide: Text.ElideRight
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeTitle
                font.weight: FiliTheme.weightAccent
                color: win.attending ? win.onAccent : FiliTheme.text
            }
            // Window buttons at the far right: close, and the panel of window actions (niri only)
            Rectangle {
                id: closeButton
                visible: win.windowButtons
                anchors { right: parent.right; rightMargin: (toolbar.height - height) / 2; verticalCenter: parent.verticalCenter }
                width: 28
                height: 24
                radius: FiliTheme.radiusControl
                color: closeArea.pressed ? FiliTheme.danger : closeArea.containsMouse ? FiliTheme.alpha(FiliTheme.danger, 0.75) : "transparent"
                WindowGlyph { anchors.centerIn: parent; kind: "close"; width: 16; height: 16; color: closeArea.containsMouse ? "#ffffff" : win.attending ? win.onAccent : FiliTheme.text }
                MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; onClicked: win.quit() }
                Accessible.role: Accessible.Button
                Accessible.name: Fili.tr("Close window")
            }
            Rectangle {
                id: windowButton
                visible: win.windowButtons
                anchors { right: closeButton.left; rightMargin: FiliTheme.space1; verticalCenter: parent.verticalCenter }
                width: 28
                height: 24
                radius: FiliTheme.radiusControl
                color: windowArea.pressed || windowArea.containsMouse || windowPanel.visible ? (win.attending ? win.tintHover : windowArea.pressed ? FiliTheme.surfaceActive : FiliTheme.surfaceHover) : "transparent"
                WindowGlyph { anchors.centerIn: parent; kind: "window"; width: 16; height: 16; color: win.attending ? win.onAccent : FiliTheme.text }
                MouseArea { id: windowArea; anchors.fill: parent; hoverEnabled: true; onClicked: windowPanel.visible ? windowPanel.close() : windowPanel.open() }
                Accessible.role: Accessible.Button
                Accessible.name: Fili.tr("Window")
            }
            FiliLiaButton {
                id: liaButton
                visible: Fili.liaAvailable()
                anchors { right: windowButton.visible ? windowButton.left : closeButton.visible ? closeButton.left : parent.right; rightMargin: windowButton.visible || closeButton.visible ? FiliTheme.space1 : FiliTheme.margin; verticalCenter: parent.verticalCenter }
                ink: win.attending ? win.onAccent : FiliTheme.text
            }
            FiliButton {
                id: addButton
                anchors { right: liaButton.visible ? liaButton.left : windowButton.visible ? windowButton.left : closeButton.visible ? closeButton.left : parent.right; rightMargin: liaButton.visible || windowButton.visible || closeButton.visible ? FiliTheme.space3 : FiliTheme.margin; verticalCenter: parent.verticalCenter }
                label: "+  " + Fili.tr("New")
                primary: true
                onClicked: quick.start()
            }
            // Day · week · month in a capsule, like the navigation buttons of Casier
            Rectangle {
                x: views.x - FiliTheme.space1 - 1
                anchors.verticalCenter: parent.verticalCenter
                width: views.width + 2 * FiliTheme.space1 + 2
                height: 30
                radius: 8
                color: win.attending ? win.tintSurface : FiliTheme.surface
                border.width: FiliTheme.borderWidth
                border.color: win.attending ? win.tintBorder : FiliTheme.border
            }
            Row {
                id: views
                anchors { right: addButton.left; rightMargin: FiliTheme.space4 + FiliTheme.space1 + 1; verticalCenter: parent.verticalCenter }
                spacing: FiliTheme.space1
                Repeater {
                    model: [{ id: "day", label: Fili.tr("Day") }, { id: "week", label: Fili.tr("Week") }, { id: "month", label: Fili.tr("Month") }, { id: "year", label: Fili.tr("Year") }, { id: "list", label: Fili.tr("List") }]
                    Rectangle {
                        id: seg
                        required property var modelData
                        readonly property bool on: win.viewMode === modelData.id
                        width: segText.implicitWidth + 2 * FiliTheme.space3
                        height: 22
                        radius: FiliTheme.radiusControl
                        color: on || segArea.containsMouse ? (win.attending ? win.tintHover : FiliTheme.surfaceHover) : "transparent"
                        Text {
                            id: segText
                            anchors.centerIn: parent
                            text: seg.modelData.label
                            font.family: FiliTheme.font
                            font.pixelSize: FiliTheme.sizeList
                            font.weight: seg.on ? FiliTheme.weightAccent : Font.Normal
                            color: win.attending ? win.onAccent : seg.on ? FiliTheme.accentColor : FiliTheme.textSecondary
                        }
                        MouseArea { id: segArea; anchors.fill: parent; hoverEnabled: true; onClicked: win.setView(seg.modelData.id) }
                        Accessible.role: Accessible.RadioButton
                        Accessible.name: seg.modelData.label
                    }
                }
            }
        }

        // ---- Sidebar --------------------------------------------------------------------------------------------------
        FiliSidebar {
            id: side
            anchors { top: toolbar.bottom; bottom: debug.top; left: parent.left }
            width: FiliTheme.sidebarWidth
            MiniMonth {
                width: parent.width - 2 * FiliTheme.space2
                x: FiliTheme.space2
                tools: tools
                anchor: win.anchor
                today: win.today
                events: win.events
                firstDay: win.firstDay
                onPicked: d => win.anchor = d
            }
            FiliSidebarSection {
                title: Fili.tr("Calendars")
                // The usual "+" of a list of calendars: a small menu to make one, or to add an account
                trailing: FiliIconButton {
                    id: addCalendarButton
                    name: "plus"
                    tip: Fili.tr("Add a calendar or an account")
                    onClicked: {
                        const p = addCalendarButton.mapToItem(addMenu.parent, 0, addCalendarButton.height);
                        addMenu.open(p.x - addMenu.menuWidth + addCalendarButton.width, p.y + 2, [{ id: "calendar", label: Fili.tr("New calendar…") }, { id: "account", label: Fili.tr("Add account…") }]);
                    }
                }
                Repeater {
                    model: win.calendars
                    Item {
                        id: calRow
                        required property var modelData
                        width: parent ? parent.width : 0
                        height: 28
                        activeFocusOnTab: true
                        Keys.onReturnPressed: calendarDialog.edit(calRow.modelData)
                        Rectangle {
                            anchors { fill: parent; leftMargin: FiliTheme.space2; rightMargin: FiliTheme.space2 }
                            radius: FiliTheme.radiusControl
                            color: calArea.containsMouse ? FiliTheme.surfaceHover : "transparent"
                        }
                        // The colour is a check box: shown, or hidden
                        Rectangle {
                            id: calBox
                            x: FiliTheme.margin
                            anchors.verticalCenter: parent.verticalCenter
                            width: 14
                            height: 14
                            radius: 4
                            color: calRow.modelData.hidden ? "transparent" : tools.colorOf(calRow.modelData)
                            border.width: 2
                            border.color: tools.colorOf(calRow.modelData)
                            MouseArea { anchors.fill: parent; anchors.margins: -5; onClicked: win.setCalendar(calRow.modelData.slug, "hidden", calRow.modelData.hidden ? "false" : "true") }
                            Accessible.role: Accessible.CheckBox
                            Accessible.name: calRow.modelData.name
                            Accessible.checked: !calRow.modelData.hidden
                        }
                        Text {
                            anchors { left: calBox.right; leftMargin: FiliTheme.space2; right: parent.right; rightMargin: FiliTheme.space3; verticalCenter: parent.verticalCenter }
                            text: calRow.modelData.name
                            elide: Text.ElideRight
                            font.family: FiliTheme.font
                            font.pixelSize: FiliTheme.size
                            color: calRow.modelData.hidden ? FiliTheme.muted : FiliTheme.text
                        }
                        MouseArea { id: calArea; anchors { fill: parent; leftMargin: calBox.x + calBox.width + 6 }
                            hoverEnabled: true; onClicked: calendarDialog.edit(calRow.modelData) }
                    }
                }
            }
            FiliSidebarSection {
                title: Fili.tr("Coming up")
                Repeater {
                    model: win.upcoming
                    Item {
                        id: soonRow
                        required property var modelData
                        width: parent ? parent.width : 0
                        height: 40
                        Rectangle {
                            anchors { fill: parent; leftMargin: FiliTheme.space2; rightMargin: FiliTheme.space2; topMargin: 1; bottomMargin: 1 }
                            radius: FiliTheme.radiusControl
                            color: soonArea.containsMouse ? FiliTheme.surfaceHover : "transparent"
                        }
                        Rectangle { x: FiliTheme.margin; y: 8; width: 3; height: 24; radius: 1.5; color: tools.colorOf(soonRow.modelData) }
                        Column {
                            anchors { left: parent.left; leftMargin: FiliTheme.margin + 10; right: parent.right; rightMargin: FiliTheme.space3; verticalCenter: parent.verticalCenter }
                            Text { width: parent.width; text: soonRow.modelData.title; elide: Text.ElideRight; font.family: FiliTheme.font; font.pixelSize: FiliTheme.sizeList; color: FiliTheme.text }
                            Text { width: parent.width; text: win.whenText(soonRow.modelData) + (soonRow.modelData.place ? " · " + soonRow.modelData.place : ""); elide: Text.ElideRight; font.family: FiliTheme.font; font.pixelSize: FiliTheme.sizeSmall; color: FiliTheme.muted }
                        }
                        MouseArea { id: soonArea; anchors.fill: parent; hoverEnabled: true; onClicked: win.anchor = tools.day(tools.start(soonRow.modelData)) }
                        Accessible.role: Accessible.Button
                        Accessible.name: soonRow.modelData.title
                    }
                }
                Text {
                    visible: win.upcoming.length === 0
                    x: FiliTheme.margin
                    text: Fili.tr("Nothing in the next two weeks")
                    font.family: FiliTheme.font
                    font.pixelSize: FiliTheme.sizeSmall
                    color: FiliTheme.muted
                }
            }
        }

        // ---- Content: the view ----------------------------------------------------------------------------------------
        Item {
            id: content
            anchors { top: toolbar.bottom; bottom: debug.top; left: side.right; right: parent.right }
            MonthView {
                anchors.fill: parent
                visible: win.viewMode === "month"
                tools: tools
                anchor: win.anchor
                events: win.events
                firstDay: win.firstDay
                today: win.today
                selectedKey: win.selectedKey
                onDayClicked: d => win.anchor = d
                onDayActivated: d => win.newEvent(d)
                onEventClicked: ev => { win.select(ev); editor.edit(ev); }
                onEventContext: (ev, x, y) => win.showContext(ev, x, y)
                onSlotContext: (at, x, y) => win.showSlotContext(at, x, y)
                onEventMoved: (ev, dDays, copy) => win.changeEvent(ev, dDays * 1440, dDays * 1440, copy)
                onDaysCreated: (first, last) => editor.createDays(first, last)
            }
            YearView {
                anchors.fill: parent
                visible: win.viewMode === "year"
                tools: tools
                anchor: win.anchor
                events: win.events
                today: win.today
                firstDay: win.firstDay
                onDayPicked: d => { win.anchor = d; win.setView("month"); }
            }
            AgendaList {
                anchors.fill: parent
                visible: win.viewMode === "list"
                tools: tools
                events: win.events
                today: win.today
                selectedKey: win.selectedKey
                onEventClicked: ev => { win.select(ev); editor.edit(ev); }
                onEventContext: (ev, x, y) => win.showContext(ev, x, y)
                onDoneToggled: ev => win.toggleDone(ev)
            }
            TimeGrid {
                anchors.fill: parent
                visible: win.viewMode === "day" || win.viewMode === "week"
                tools: tools
                first: win.rangeStart
                days: win.viewMode === "week" ? 7 : 1
                events: win.events
                today: win.today
                now: win.now
                selectedKey: win.selectedKey
                onSlotActivated: at => win.newEvent(at)
                onEventClicked: ev => { win.select(ev); editor.edit(ev); }
                onEventContext: (ev, x, y) => win.showContext(ev, x, y)
                onSlotContext: (at, x, y) => win.showSlotContext(at, x, y)
                onRangeCreated: (start, end) => editor.create(start, end)
                onEventChanged: (ev, dStart, dEnd, copy) => win.changeEvent(ev, dStart, dEnd, copy)
            }
        }

        // Debug panel (View > Debug Logs): the log of the window above the footer, or at the bottom when it is hidden
        FiliDebugPanel {
            id: debug
            anchors { bottom: statusbar.top; left: parent.left; right: parent.right }
        }

        // ---- Footer ---------------------------------------------------------------------------------------------------
        Rectangle {
            id: statusbar
            anchors.bottom: parent.bottom
            width: parent.width
            height: win.footerShown ? FiliTheme.statusHeight : 0
            visible: win.footerShown
            // Recessed like Facteur's footer (app-window.md of the filigrane-ui skill)
            color: win.attending ? FiliTheme.accentColor : FiliTheme.backgroundDeep
            MouseArea { anchors.fill: parent; z: -1; enabled: win.attending; onClicked: win.attentionText = "" }
            Text {
                anchors { fill: parent; leftMargin: FiliTheme.space3; rightMargin: FiliTheme.space3; topMargin: 1 }
                z: 1
                visible: win.attending
                verticalAlignment: Text.AlignVCenter
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideMiddle
                text: win.attentionText
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                font.weight: FiliTheme.weightAccent
                color: win.onAccent
            }
            Rectangle { width: parent.width; height: FiliTheme.borderWidth; color: FiliTheme.border }
            // An error or a notice takes the whole bar
            Text {
                anchors { fill: parent; leftMargin: FiliTheme.space3; rightMargin: FiliTheme.space3; topMargin: 1 }
                visible: cal.message !== "" && !win.attending
                verticalAlignment: Text.AlignVCenter
                horizontalAlignment: Text.AlignHCenter
                elide: Text.ElideMiddle
                text: cal.message
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: cal.failed ? FiliTheme.danger : FiliTheme.textSecondary
                MouseArea { anchors.fill: parent; onClicked: cal.clearMessage() }
            }
            Text {
                anchors { left: parent.left; leftMargin: FiliTheme.margin; right: parent.horizontalCenter; top: parent.top; topMargin: 1; bottom: parent.bottom }
                visible: cal.message === "" && !win.attending
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
                text: win.events.length + " " + Fili.tr("events") + " · " + win.periodTitle
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: FiliTheme.textSecondary
            }
            Text {
                anchors { right: parent.right; rightMargin: FiliTheme.margin; left: parent.horizontalCenter; top: parent.top; topMargin: 1; bottom: parent.bottom }
                visible: cal.message === "" && !win.attending
                verticalAlignment: Text.AlignVCenter
                horizontalAlignment: Text.AlignRight
                elide: Text.ElideMiddle
                text: Fili.tr("Double click to add")
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: FiliTheme.muted
            }
        }

        // ---- Quick add: one line, "tomorrow 14:30 Dentist" ------------------------------------------------------------
        FiliDialog {
            id: quick
            title: Fili.tr("New event")
            confirmLabel: Fili.tr("Add")
            cardWidth: 480
            readonly property var parsed: quickField.text.trim() === "" ? ({}) : JSON.parse(cal.parseQuick(quickField.text))
            readonly property bool ok: parsed.title !== undefined
            confirmEnabled: ok
            function start() {
                quickField.text = "";
                open();
                quickField.focusField();
            }
            onAccepted: {
                cal.add(JSON.stringify(parsed));
                soon.refresh();
            }
            FiliTextField {
                id: quickField
                width: parent.width
                placeholder: Fili.tr("tomorrow 14:30 Dentist · +30m Take the bread out · 2026-12-24 Christmas Eve")
                onAccepted: quick.accept()
            }
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: quick.ok ? (quick.parsed.reminder ? Fili.tr("Reminder") + " · " : "") + quick.parsed.title + "  —  " + (quick.parsed.allDay ? tools.parse(quick.parsed.start).toLocaleDateString(Qt.locale(), "dddd d MMMM") : tools.parse(quick.parsed.start).toLocaleString(Qt.locale(), "dddd d MMMM, HH:mm")) : Fili.tr("Start with when: 14:30, tomorrow 09:00, monday 18:00, 2026-12-24, +30m")
                font.family: FiliTheme.font
                font.pixelSize: FiliTheme.sizeSmall
                color: quick.ok ? FiliTheme.textSecondary : FiliTheme.muted
            }
            FiliButton {
                label: Fili.tr("More options…")
                onClicked: {
                    const p = quick.ok ? quick.parsed : null;
                    quick.close();
                    win.newEvent(p ? tools.parse(p.start) : undefined);
                }
            }
        }

        SearchDialog {
            id: searchDialog
            tools: tools
            backend: cal
            onPicked: (day, id) => { win.anchor = day; if (win.viewMode === "year") win.setView("month"); }
        }
        ImportDialog {
            id: importDialog
            backend: cal
            calendars: win.calendars
            onImported: (source, calendar) => { cal.importSource(source, calendar); soon.refresh(); }
            onAnswered: (source, answer, calendar) => { cal.replyInvite(source, answer, calendar); soon.refresh(); }
        }
        ExportDialog {
            id: exportDialog
            calendars: win.calendars
            onExported: (calendar, path) => cal.exportTo(calendar, path)
        }
        FiliMenu {
            id: ctxMenu
            onActivated: id => {
                const ev = win.ctxEvent;
                if (id === "edit" && ev) editor.edit(ev);
                else if (id === "duplicate" && ev) win.duplicateEvent(ev);
                else if (id === "copy" && ev) win.copyEvent(ev);
                else if (id === "done" && ev) win.toggleDone(ev);
                else if (id === "delete" && ev) win.deleteEvent(ev, false);
                else if (id === "delete-series" && ev) win.deleteEvent(ev, true);
                else if (id === "new") win.newEvent(win.ctxDay);
                else if (id === "paste") win.pasteAt(win.ctxDay);
            }
        }
        FiliMenu {
            id: addMenu
            onActivated: id => { if (id === "calendar") calendarDialog.create(); else accountDialog.start(); }
        }
        AccountDialog {
            id: accountDialog
            onConnect: (address, user, password) => cal.accountConnect(address, user, password)
            onSubscribe: (name, link) => { cal.calendarAdd(name, "", link); win.reloadCalendars(); }
        }
        CalendarDialog {
            id: calendarDialog
            tools: tools
            onSaved: (slug, name, color, url) => win.saveCalendar(slug, name, color, url)
            onRefreshRequested: win.runAction("refresh-calendars")
            onDeleted: slug => { if (cal.calendarRemove(slug)) { win.reloadCalendars(); soon.refresh(); } }
        }
        EventEditor {
            id: editor
            tools: tools
            onVisibleChanged: Qt.callLater(win.publishContext)
            calendars: win.calendars
            onSaved: (id, json) => win.applySaved(id, json)
            onDeleted: (id, occ, all) => win.applyDeleted(id, occ, all)
        }

        // ---- Window actions, About ------------------------------------------------------------------------------------
        FiliWindowPanel {
            id: windowPanel
            anchorRight: width - (toolbar.height - closeButton.height) / 2
            anchorTop: toolbar.height + 1
        }

        // About card: in front of the window, closed by Esc or a click outside
        Rectangle {
            id: about
            visible: win.aboutOpen
            anchors.fill: parent
            z: 300
            color: FiliTheme.alpha(FiliTheme.background, 0.6)
            focus: visible
            MouseArea { anchors.fill: parent; onClicked: win.aboutOpen = false }
            Rectangle {
                id: card
                anchors.centerIn: parent
                width: Math.min(440, parent.width - 2 * FiliTheme.margin)
                height: cardColumn.implicitHeight + 2 * FiliTheme.space6
                radius: FiliTheme.radiusWindow
                color: FiliTheme.surface
                border.width: FiliTheme.borderWidth
                border.color: FiliTheme.border
                MouseArea { anchors.fill: parent }
                Column {
                    id: cardColumn
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: FiliTheme.space6 }
                    spacing: FiliTheme.space2
                    Image {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 72
                        height: 72
                        source: "qrc:/qt/qml/Fili/Agenda/icons/fili-cal.svg"
                        sourceSize: Qt.size(144, 144)
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Agenda"
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.sizeTitle + 4
                        font.weight: Font.Light
                        color: FiliTheme.text
                    }
                    Text {
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.Wrap
                        text: Fili.tr("Your calendar and your reminders, in one list")
                        font.family: FiliTheme.font
                        font.pixelSize: FiliTheme.size
                        color: FiliTheme.textSecondary
                    }
                    Item { width: 1; height: FiliTheme.space2 }
                    Repeater {
                        model: win.aboutOpen ? JSON.parse(Fili.aboutRows()) : []
                        Row {
                            id: aboutRow
                            required property var modelData
                            width: cardColumn.width
                            spacing: FiliTheme.space3
                            Text { width: 120; horizontalAlignment: Text.AlignRight; text: aboutRow.modelData[0]; font.family: FiliTheme.font; font.pixelSize: FiliTheme.size; color: FiliTheme.muted }
                            Text { width: parent.width - 120 - FiliTheme.space3; text: aboutRow.modelData[1]; elide: Text.ElideMiddle; font.family: FiliTheme.font; font.pixelSize: FiliTheme.size; color: FiliTheme.text }
                        }
                    }
                }
            }
        }
    }
}
