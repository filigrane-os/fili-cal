# Agenda (fili-cal)

The calendar and the reminders of [Filigrane](https://github.com/filigrane-os/filigrane), in one list: a reminder is an event
with an alert at its start. A Qt Quick window in the style of iCal, a little more modern: day, week, month, year and list views,
a small month and what comes next in the sidebar, a one-line "quick add" (`tomorrow 14:30 Dentist`, `+30m Take the bread out`),
drag to create, move and resize, search, undo, several calendars with the colours of the theme, alerts with Snooze and Open.

- Events are iCalendar files, one folder per calendar in `~/.local/share/filigrane/calendar/` (one `.ics` each): readable by any
  calendar program. Recurrence rules (`BYDAY`, `BYSETPOS`…), time zones, several alarms and the properties Agenda does not use are
  read and written back without loss.
- **Accounts without a daemon**: CalDAV (Nextcloud, Fastmail, iCloud, any server; push and pull with `ETag`s, password in the
  keyring) and read-only subscriptions by link (Proton, Google's secret address, any `.ics`), both through `curl`, started by
  `fili-cal tick`. Import and export of `.ics`, and Agenda is the handler of `text/calendar` and `webcal:`.
- No daemon: the fili-bar runs `fili-cal tick` every minute (alerts, fetches that are due, the file read by the clock card).
- The same program is the command line: `fili-cal add|quick|edit|rm|done|show|list|next|free|calendar|import|export|undo|redo|tick`
  (see `src/cli.rs`); Lia uses it to add and read events and to find free time (`shell/lia/fili-cal.md`).
- `fili-cal ctl action NAME` drives the open window (its menu in the fili-bar is `shell/appmenu/fili-cal.json`); the window tells
  `fili-context` what it shows.
- Settings: `~/.config/filigrane/fili-cal.toml`, section `[agenda]` ("Settings…" creates it with every key commented).
- Network: the place field suggests real places as you type by sending that text to a Photon server (OpenStreetMap data,
  `photon.komoot.io` by default). A line under the field says so, and the compass button next to it turns this off
  (`place_suggestions = false`); `place_search` points to another server, or `"none"`. Nothing else is sent anywhere except the
  calendars you subscribe to or connect.

```
make dev && make run        # fast build, QML read from the sources
make test                   # unit tests, the check of the bar menu, translations, the control catalog
make test-window            # drives the window offscreen through `fili-cal ctl` (after make dev)
sudo make install           # PREFIX=/usr for a package
fili-cal --view month --shot agenda.png     # picture of the window without a screen (QT_QPA_PLATFORM=offscreen)
```

GPL-3.0-or-later. The interface comes from the [fili-ui](https://github.com/filigrane-os/fili-ui) kit.

## Weight

Measured on each release, the window open and idle (resident memory, CPU averaged over 10 s).

| Version | Window RAM (RSS) | CPU idle |
|---|---|---|
| 0.2.2 | 122 MB | 1.0 % (just opened) |
