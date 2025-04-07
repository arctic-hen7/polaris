#!/usr/bin/env python
# A scheduling script that takes the object of action items from stdin and prints an ICS file
# containing all action items with timestamps. This is intended to be composed with other scripts
# that filter those items.

import re
import json
import sys
from ics import Calendar, Event
from datetime import datetime

def cal_to_ics(events):
    """
    Converts the given list of action items to an ICS calendar string.
    """

    calendar = Calendar()
    for event in events:
        # Form the body from the regular body and the associated people, if there are any
        body = event["body"] or ""
        if event["people"]:
            body += "\n\nPeople: \n- " + "\n- ".join([name for _, name in event["people"]])

        ts_start = datetime.strptime(event["timestamp"]["start"]["date"], "%Y-%m-%d")
        ts_end = ts_start
        if event["timestamp"]["start"]["time"]:
            ts_start = ts_start.replace(hour=int(event["timestamp"]["start"]["time"][:2]), minute=int(event["timestamp"]["start"]["time"][3:5]))
        if event["timestamp"]["end"]:
            ts_end = datetime.strptime(event["timestamp"]["end"]["date"], "%Y-%m-%d")
            if event["timestamp"]["end"]["time"]:
                ts_end = ts_end.replace(hour=int(event["timestamp"]["end"]["time"][:2]), minute=int(event["timestamp"]["end"]["time"][3:5]))

        ev = Event(
            event["title"],
            begin=ts_start,
            description=body.strip()
        )
        if ts_end:
            ev.end = ts_end
        if not event["timestamp"]["start"]["time"] and not event["timestamp"]["end"]:
            ev.make_all_day()
        if event["location"]:
            ev.location = event["location"]

        calendar.events.add(ev)

    ics_str = calendar.serialize()
    # Remove all UTC timezone specifications (in DTSTART and DTEND properties)
    ics_str = re.sub(r'(DTSTART:\d+T\d+)Z', r'\1', ics_str)
    ics_str = re.sub(r'(DTEND:\d+T\d+)Z', r'\1', ics_str)

    return ics_str

if __name__ == "__main__":
    data = json.load(sys.stdin)

    ics_str = cal_to_ics(data["events"])
    print(ics_str)
