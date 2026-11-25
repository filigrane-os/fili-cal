# Agenda (fili-cal): plan

Where it stands (0.1.0, local repository, not published): one list of events and reminders (an `.ics` file each), day, week and
month views, a small month and "Coming up", quick add, an editor, alerts through `fili-cal tick`, the command line used by the
fili-bar and Lia. Goal: an application as pleasant as iCal for daily use, a little more modern, light, and open (files, no
account, no daemon).

Rules that hold for every step: the interface comes from the `fili-ui` kit (a missing piece is added to the kit first); no
shader effect; every action is reachable from the keyboard and from the fili-bar menu (`shell/appmenu`, `ctl` catalog); a
fix or a feature comes with its test; the CPU and RAM of each release go in the README of the distribution; the text never
overflows (check on a capture: `fili-cal --view month --shot f.png` with `QT_QPA_PLATFORM=offscreen`).

## P0. Debt to settle first (correctness, nothing visible)

These make a real calendar file import wrongly today; they come before any new feature.

1. **Recurrence rules**: `BYDAY` (every Monday and Thursday, the second Tuesday), `BYMONTHDAY`, `BYMONTH`, `BYSETPOS`, `WKST`,
   `RDATE`. Today only `FREQ`, `INTERVAL`, `COUNT`, `UNTIL` are read and the rest is dropped silently: an imported weekly event
   on two days shows on one. Unknown rule parts must not be dropped: keep the original `RRULE` text and write it back, and say
   in the editor that the rule is "custom" when it cannot be shown.
2. **Time zones**: events are "floating" (the hour on the wall) and `TZID` is ignored on import, so an event from another zone
   is shifted. Read `TZID` with the system zone database (`/usr/share/zoneinfo`), keep it, convert for display, and let the
   editor choose a zone per event ("Floating" stays the default). A recurrence stays at the same local hour across daylight saving.
3. **Whole-day events across several days** (`DTEND` exclusive), and `DURATION`.
4. **Alarms**: only `DISPLAY` with a relative trigger. Read absolute triggers (`TRIGGER;VALUE=DATE-TIME`), `REPEAT`/`DURATION`
   and several alarms; the editor shows all of them (today it keeps only the first).
5. **Robust files**: an `.ics` file that cannot be read is listed in the footer instead of disappearing; writes keep unknown
   properties (`ATTENDEE`, `CATEGORIES`, `X-*`) of an imported event. A cache of the parsed store (modification times) so
   that `tick`, run every minute, stays cheap with thousands of events.
6. **Tests**: a folder of real calendars (anonymised exports from Proton, Google, Apple, Thunderbird) read and written back
   without loss; recurrence cases from RFC 5545 section 3.8.5; daylight-saving days; a property test "to_ics then parse is the identity".

Done when: the fixtures round-trip, and `fili-cal list` over a Google export matches what the source application shows.

## P1. Daily use: direct manipulation

1. **Create by dragging** on the hours (day and week): the block follows the pointer in steps of 15 minutes, the editor opens
   with the title field selected. In the month view, dragging over days makes a multi-day event.
2. **Move and resize**: drag a block to another time or day, drag its bottom edge to change the length; in the month view,
   drag a bar to another day. Alt copies. Escape cancels while dragging. A series asks nothing: the move applies to the
   occurrence's series from the dragged one (a choice "this event / all events" appears only on release when it is ambiguous).
3. **Undo and redo** (Ctrl+Z, Ctrl+Shift+Z) for every change made by the window or by `fili-cal`, with a journal of the store
   like `fili-tx` (one request = one transaction, `fili-cal undo` for Lia and scripts: "fili-cal undo" is what Lia says after a change).
4. **Context menu** (right click): Edit, Duplicate, Delete, Copy, Paste (Ctrl+C / Ctrl+V of events, also as text for a chat).
5. **Search** (Ctrl+F, header field like Casier's): title, place, notes, over all time, results as a list, Enter goes to the day.
6. **Keyboard**: arrows move the selected day (month) or hour (week), Enter opens, Delete removes, `T` today, `1`/`2`/`3` views.
   A visible focus on the selected cell, event selection with Tab in reading order.
7. **Settings** (`~/.config/filigrane/fili-cal.toml`, a "Settings…" entry in the bar menu): first day of the week (default: the
   language's), 12 or 24 hours, default length, default alert, first hour shown, week numbers, show declined/hidden calendars.

## P2. Calendars and colours

1. **Several calendars**: a folder per calendar under `~/.local/share/filigrane/calendar/<name>/` (the current flat folder is
   migrated into "Personal" at the first start), each with a name and a colour (the accent tones of the theme; one more
   colour never replaces the accent of the interface for UI elements, only event marks).
2. **Sidebar** "Calendars" section, foldable like Facteur's: a check box shows or hides each one, a click on the colour changes
   it, "New calendar…", rename, delete (to the trash, undoable). The editor gets a "Calendar" choice.
3. `fili-cal add --calendar NAME`, `list --calendar`, JSON `calendar` and `color` fields; Lia's brief updated.
4. A "Birthdays" calendar from the address book later (P4), read-only.

## P3. Sources: subscriptions and accounts

1. **ICS subscriptions** (read-only): URL, refresh every N minutes by `fili-cal tick` (conditional requests with `ETag` / `Last-Modified`,
   an offline copy kept). This replaces the user-layer `cal-notify` / Proton plugin: the Proton private link becomes a
   subscription in the user layer (`users/<login>/`), never in the distribution. The clock card keeps reading the same agenda file.
2. **CalDAV** (read and write; Proton through its bridge, Nextcloud, Fastmail, iCloud): discovery, `ctag`/`sync-token` sync,
   conflict = the server wins and the local copy goes to a `.conflict` file. Passwords in the keyring only, never in a file of
   the repository or of `~/.config`. Written in Rust without a resident process: sync at the start, on a manual "Refresh", and
   from `tick` at a slow pace. Decide before starting whether to write the client or use `vdirsyncer` (a Python daemon: against the weight rules).
3. **Import and export** of `.ics` by the menu and by drag and drop of a file on the window; an invitation file opened from
   Facteur (`text/calendar`) opens a preview with "Add to calendar".
4. Default application: Agenda becomes the handler of `text/calendar` and `webcal:` (the "Default applications" page already lists it).

## P4. More views

1. **Year**: twelve small months, a heat of busy days, a click goes to the month.
2. **List** ("Agenda" view): the next days as a scrolling list grouped by day, the quickest way to read a week.
3. **Week numbers** column, **work week** (hide the weekend), a **two or three days** view.
4. **Events with a time in a short month cell** show a dot list when the cell is narrow; "+n more" opens a small popover of the day.
5. **Reminders without a time** ("Do this today"): an event of the whole day with a check box that strikes it through; they sit
   in a "To do" strip of the day and week views. Still the same list and the same files (`VTODO` is not introduced: a
   completed reminder carries `STATUS:COMPLETED` on its event).
6. **Birthdays** from the contacts of Facteur when it has them.

## P5. Alerts that help

1. Notification buttons **Snooze 10 min** (and 1 hour, tomorrow), **Open**, **Done** (the notification of the shell supports buttons, as
   `fili-incident` does); the snooze is kept in the state folder, not in the event.
2. **Alerts for whole days** at a chosen hour (default 09:00) instead of the fixed one.
3. **Travel time** (an alert earlier than the event) for events that have a place, as an option.
4. **Missed alerts** after a suspend or a shutdown: shown once, grouped, never a storm (catch-up is 12 hours today).
5. `fili-event emit` on each alert (`calendar.alert`) so that other tools can react; the bar module shows the next event.

## P6. Integration with Filigrane

1. **Clock card** of the fili-bar: a click opens Agenda on today (the small month popup stays for a quick look, with a
   "New event or reminder" and an "Open Agenda" button); the card lists the next events from the agenda file (already done).
2. **Context for Lia**: Agenda writes `fili-context` (`about`, the shown period, the selected day and event) so that "this
   event", "next week" work with Mod+Y; `ctl` gains `goto DATE`, `select ID`, `search TEXT`.
3. **Lia can plan**: "find a free hour on Thursday afternoon" through a read-only `fili-cal free --on DATE --for 1h`; adding
   stays an explicit request from the user.
4. **Casier**: a file dropped on an event as an attachment link (`ATTACH` with a `file:` URL), shown in the editor.
5. **Facteur**: "Add to calendar" on an invitation, and a reply (accept, decline) once CalDAV exists.
6. Launcher entry, `Mod+C`-style shortcut (to choose with the user), `webcal:` and `text/calendar` handlers.

## P7. Quality, weight, release

1. **Tests**: `cargo test` (store, recurrence, time zones, JSON model), a shell check of the command line, QML tests that drive
   the window (open the editor, drag a block) the way `make test-tools` does for photocraft, captures of the three views compared
   with references in `tests/shots/` (offscreen).
2. **Weight**: CPU and RAM at rest and while the week view scrolls, noted in the README for every release; target under 60 MB
   resident, no timer faster than 30 s while the window is hidden.
3. **Accessibility**: every control named, the grid readable by AT-SPI (day cells and events as list items), keyboard only.
4. **HIG review** with the `filigrane-ui` checklist (anatomy check `tests/fili-shell/check-app-window.py`, header 44, sidebar 200,
   footer 28, fili-ui tokens only).
5. **Release**: CI like the other apps (`make check` in a Void container, tag from `Cargo.toml` + `CHANGELOG.md`), the repository
   `filigrane-os/fili-cal` published with the user's agreement, the pin `apps/fili-cal/upstream` in the distribution, the
   copy of the command line in `apps/fili-cal` removed at that moment, the package `fili-apps` rebuilt, then a live test by the user
   before the single tag.

## Order and size

| Step | What | Size |
|---|---|---|
| 1 | P0 (rules, zones, files, fixtures) | large, a week of evenings |
| 2 | P1.1 to P1.3 (drag, move, undo), P1.7 (settings) | large |
| 3 | P2 (calendars, colours) | medium |
| 4 | P3.1 and P3.3 (subscriptions, import/export), P5.1 and P5.2 | medium |
| 5 | P1.4 to P1.6, P4.1 to P4.4 (search, context menu, year, list) | medium |
| 6 | P6.1 and P6.2 (clock card, context for Lia) | small |
| 7 | P3.2 (CalDAV), P4.5 (reminders without a time), P6.3 to P6.5 | large, decide after live use |

Release 0.2.0 after steps 1 and 2 (the first one worth publishing), 0.3.0 after step 4, 1.0.0 once CalDAV or subscriptions
replace the personal link and the anatomy, accessibility and weight checks pass.

## Open decisions (for the user)

- Name: "Agenda" (the display name) or "Calendrier"; the program stays `fili-cal`.
- CalDAV: write a small client in Rust, or only subscriptions (read-only) at first.
- Reminders without a time: a check box on a whole-day event (proposed), or a separate "To do" page (rejected so far: one list).
- Colours: one colour per calendar from the accent tones (proposed), or free colours.
