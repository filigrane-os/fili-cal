# Agenda (`fili-cal`)

Agenda is the calendar and the reminders of the user: one list, a reminder is an event with an alert at its start. The events are iCalendar files in `~/.local/share/filigrane/calendar/`.
- Read and write events with the command line, not with the window: `fili-cal list [--days N] [--json]`, `fili-cal next [N]`, `fili-cal add TITLE --at WHEN [--for 1h] [--place P] [--alarm 10m|none] [--every day|week|month|year]`, `fili-cal quick "tomorrow 14:30 Dentist"`, `fili-cal edit ID ...`, `fili-cal rm ID [--on WHEN]`.
- Calendars: one folder per calendar; `fili-cal calendar list|add NAME [TONE]|rename|color|hide|show|rm`, `--calendar NAME` on `add`, `quick`, `edit` and `list` (JSON has `calendar`). Without it an event goes to "Personal". After a change, say `fili-cal undo` undoes it.
- WHEN: `+2h`, `14:30`, `2026-12-24` (whole day), `2026-12-24 20:00`, `tomorrow 09:00`, `monday 18:00`. Turn the user's words into one of these yourself, and say the date you chose.
- `fili-cal ctl [--pid PID] action NAME` drives the open window (view-day, view-week, view-month, today, previous, next, new-event, quick-add): the same actions as its menu.
- Adding is what the user asked for: do it. Every change is a step of the journal (`fili-cal undo`, `redo`); still, delete only the one the user names, and say which one (`fili-cal show ID`).
- Events can hold private details (places, notes): never send them outside the machine.
