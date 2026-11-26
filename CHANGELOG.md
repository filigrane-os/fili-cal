# Changelog

## 0.2.1

- `package.env` for the xbps package of Filigrane: the Qt, curl, keyring, notification and clipboard dependencies, and the
  `agenda` alias.

## 0.2.0

Agenda becomes a calendar you can live in.

- Correct files: recurrence rules (`BYDAY`, `BYMONTHDAY`, `BYMONTH`, `BYSETPOS`, `WKST`, `RDATE`), time zones (`TZID`, UTC), whole-day
  events over several days, `DURATION`, absolute, repeated and end-related alarms, changed occurrences of a series, and the properties
  Agenda does not use are kept. A file that cannot be read is told in the footer.
- Direct manipulation: drag to create, move and resize in the day and week views (Alt copies, Escape cancels), drag bars between days
  in the month, drag over days to make a multi-day event, undo and redo (Ctrl+Z, `fili-cal undo`), right-click menus, copy, paste and
  duplicate, keyboard (T, 1 to 5, arrows, Enter, Delete).
- Several calendars, one folder each, with a name and a colour from the theme; a sidebar section with a "+" menu.
- Subscriptions by link (read-only, `ETag`, offline copy) and CalDAV accounts (Nextcloud, Fastmail, iCloud, others; keyring password;
  push and pull; the server wins on a conflict); an "Add account" dialog with the account types.
- Import and export of `.ics`, a file or `webcal:` link given to `fili-cal`, handler of `text/calendar` and `webcal:`.
- Year and list views, search (Ctrl+F), week numbers, work week, a "+n more" popover, whole-day events that can be ticked as done,
  attached files.
- Alerts: Snooze 10 min, Snooze 1 hour, Open; the alerts missed during a suspend as one notice; whole-day alert hour and travel time
  as settings; `fili-event emit calendar.alert`.
- The notes are a text field of several lines; on the card their links can be clicked, and every video meeting link (Teams, Meet, Zoom…)
  is taken out of them into a Join button (the options page of an invitation is not taken for the meeting).
- As many alerts as wanted for an event (a line each in the editor, usual choices or the ones of the file, "Add an alert").
- Joining opens the link in the browser and brings the browser to the front (niri).
- An event opens as a card to read (title, when, repeat, alert, the Join button, place, notes, attachments); the Edit button switches to the form.
- Video meetings: the link of Teams, Meet, Zoom, Webex… found in the place, notes or properties gives a Join button (week, month, list, editor,
  sidebar, right-click menu, Ctrl+J), a Join button in the alert (a click on the bubble joins) and `fili-cal join`.
- Invitations: Accept, Maybe, Decline in the import dialog, answered by Facteur; the clock card of the bar marks the days with events,
  lists the day selected and joins meetings (`fili-cal --goto DAY`).
- Settings in `fili-cal.toml` (first day, 12 or 24 hours, default length and alert, first hour, work week, week numbers, travel time).
- The place field suggests real places as you type (Photon, OpenStreetMap). A line under the field says that the text goes to
  `photon.komoot.io`, and the compass button next to it turns the suggestions off (`place_suggestions`; `place_search` sets the server).
- `fili-cal --new JSON` (and the `new-from` action) opens the editor of a new event, filled and not saved: Facteur's "Add to Agenda".
- An invitation handed over by Facteur or the system opens without the file field, its button says "Add without answering".
- For Lia and scripts: the window publishes its context, `goto`, `select` and `search` actions, `fili-cal free`.
- `make test-window` drives the window offscreen.

## 0.1.0

First version: the calendar and the reminders in one list. Day, week and month views, a small month and "Coming up" in the
sidebar, a quick add line and an event editor (title, whole day, date and times, repeat, alert, place, notes), repeating events
(day, week, month, year) with the deletion of one occurrence, alerts as notifications, iCalendar files, the command line used by
the fili-bar and Lia.
