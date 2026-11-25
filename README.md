# Agenda (fili-cal)

The calendar and the reminders of [Filigrane](https://github.com/filigrane-os/filigrane), in one list: a reminder is an event
with an alert at its start. A Qt Quick window in the style of iCal, a little more modern: day, week and month, a small month
and what comes next in the sidebar, a one-line "quick add" (`tomorrow 14:30 Dentist`, `+30m Take the bread out`), alerts as
notifications.

- Events are iCalendar files in `~/.local/share/filigrane/calendar/` (one `.ics` each): readable by any calendar program,
  `fili-cal import` / `export` move them in and out.
- No daemon: the fili-bar runs `fili-cal tick` every minute (alerts, and the file read by the clock card).
- The same program is the command line: `fili-cal add|quick|edit|rm|show|list|next|tick|import|export` (see `src/cli.rs`); Lia
  uses it to add and read events (`shell/lia/fili-cal.md`).
- `fili-cal ctl action NAME` drives the open window (its menu in the fili-bar is `shell/appmenu/fili-cal.json`).

```
make dev && make run        # fast build, QML read from the sources
make test                   # unit tests, the check of the bar menu, translations, the control catalog
sudo make install           # PREFIX=/usr for a package
fili-cal --view month --shot agenda.png     # picture of the window without a screen (QT_QPA_PLATFORM=offscreen)
```

GPL-3.0-or-later. The interface comes from the [fili-ui](https://github.com/filigrane-os/fili-ui) kit.
