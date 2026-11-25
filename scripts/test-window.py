#!/usr/bin/env python3
"""Drives the window without a mouse or a screen: starts it offscreen with a store of its own, then talks to it through
`fili-cal ctl` (the same commands as the menu of the bar and the assistant). What it checks: views and navigation, selecting an
event opens the editor, a file given to a second `fili-cal` reaches the open window as an import, the search dialog, the actions
that are disabled while a dialog is open.

    make dev && python3 -I scripts/test-window.py            (BIN=path to another build)
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
binary = os.environ.get("BIN") or os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(root, "target")), "fast", "fili-cal")
if not os.access(binary, os.X_OK):
    sys.exit(f"build first (make dev): {binary}")

tmp = tempfile.mkdtemp(prefix="fili-cal-window-")
run = os.path.join(tmp, "run")
os.makedirs(run, mode=0o700)
env = dict(os.environ, XDG_RUNTIME_DIR=run, XDG_DATA_HOME=os.path.join(tmp, "data"), XDG_STATE_HOME=os.path.join(tmp, "state"),
           XDG_CONFIG_HOME=os.path.join(tmp, "config"), QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software",
           FILI_CAL_NOW="2026-12-01T10:00", FILI_CAL_NOTIFY="true", FILI_QML_DIR=os.path.join(root, "qml"))
ui = os.path.join(root, "..", "fili-ui", "qml")
if os.path.isdir(ui):
    env["FILI_UI_QML"] = os.path.abspath(ui)

failures = []


def check(name, ok, detail=""):
    print(("ok - " if ok else "not ok - ") + name + ("" if ok else f"  ({detail})"))
    if not ok:
        failures.append(name)


def cli(*args, stdin=None):
    return subprocess.run([binary, *args], env=env, capture_output=True, text=True, timeout=30, input=stdin)


def ctl(*args):
    r = cli("ctl", *args)
    try:
        return json.loads(r.stdout)
    except ValueError:
        return {"ok": False, "error": r.stdout + r.stderr}


def state():
    return ctl("state")


def until(fn, seconds=8.0):
    end = time.time() + seconds
    while time.time() < end:
        v = fn()
        if v:
            return v
        time.sleep(0.1)
    return None


# Events: one of today, one far away, one in a series
ids = {}
for name, args in {
    "dentist": ["add", "Dentist", "--at", "2026-12-03 09:30", "--place", "Rue 1"],
    "party": ["add", "Christmas party", "--at", "2026-12-24 20:00"],
}.items():
    ids[name] = cli(*args).stdout.strip()
check("the events were made", all(ids.values()), ids)

app = subprocess.Popen([binary, "--view", "week"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    st = until(lambda: (lambda s: s if s.get("ok") else None)(state()), 20)
    check("the window starts and answers", st is not None)
    if st is None:
        raise SystemExit(1)
    check("the week view is the one asked", st["view"] == "week", st)

    for view in ("day", "month", "year", "list", "week"):
        ctl("action", f"view-{view}")
        check(f"view-{view}", until(lambda: state().get("view") == view), state())

    ctl("action", "goto 2026-12-24")
    check("goto moves the day", until(lambda: state().get("date") == "2026-12-24"), state())
    ctl("action", "view-month")
    until(lambda: state().get("view") == "month")
    check("the month shows the party", until(lambda: state().get("events", 0) >= 1), state())
    ctl("action", "next")
    check("next goes to the next month", until(lambda: state().get("date", "").startswith("2027-01")), state())
    ctl("action", "today")
    ctl("action", "view-week")

    # select looks in the events that are loaded: go to the day first, then select
    ctl("action", "goto 2026-12-03")
    until(lambda: state().get("date") == "2026-12-03")
    time.sleep(0.5)
    ctl("action", f"select {ids['dentist']}")
    check("selecting an event opens the editor", until(lambda: state().get("editing")), state())
    check("new-event is disabled while the editor is open", not ctl("action", "new-event").get("ok"), "answered ok")

    r = ctl("actions")
    names = [a["name"] for a in r.get("actions", [])]
    check("the actions are listed", {"search", "import", "export", "view-year", "undo"} <= set(names), names)

    # An iCalendar file for the next window
    invite = os.path.join(tmp, "invite.ics")
    with open(invite, "w") as f:
        f.write("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:inv@x\r\nDTSTART:20261210T150000\r\nSUMMARY:Lunch\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n")
finally:
    app.terminate()
    try:
        app.wait(timeout=5)
    except subprocess.TimeoutExpired:
        app.kill()

# A second window for the import (the first one has its editor open: a file would be an attachment there)
app = subprocess.Popen([binary, "--view", "month"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    until(lambda: state().get("ok"), 20)
    r = cli(invite)
    check("a file given to fili-cal reaches the open window", until(lambda: state().get("importing")), (state(), r.stderr))
    ctl("action", "search")
    check("search opens its dialog", until(lambda: state().get("searching")), state())
    check("quick-add is disabled while a dialog is open", not ctl("action", "quick-add").get("ok"))
finally:
    app.terminate()
    try:
        app.wait(timeout=5)
    except subprocess.TimeoutExpired:
        app.kill()
    shutil.rmtree(tmp, ignore_errors=True)

print(f"# end: {len(failures)} failure(s)")
sys.exit(1 if failures else 0)
