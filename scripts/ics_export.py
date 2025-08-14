#!/usr/bin/env python
# A scheduling script that takes an array of action items from stdin and prints an ICS file
# containing all action items with timestamps. This is intended to be composed with other scripts
# that filter those items.

import json
import sys
from ics import Calendar, Event
from datetime import datetime
from zoneinfo import ZoneInfo
from tzlocal import get_localzone

LOCAL_TZ = get_localzone()
UTC_VTIMEZONE = """\
BEGIN:VTIMEZONE
TZID:UTC
BEGIN:STANDARD
DTSTART:19700101T000000Z
TZOFFSETFROM:+0000
TZOFFSETTO:+0000
END:STANDARD
END:VTIMEZONE
"""

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

        ts_start_raw = datetime.strptime(event["timestamp"]["start"]["date"], "%Y-%m-%d")
        ts_end_raw = ts_start_raw
        if event["timestamp"]["start"]["time"]:
            ts_start_raw = ts_start_raw.replace(hour=int(event["timestamp"]["start"]["time"][:2]), minute=int(event["timestamp"]["start"]["time"][3:5]))
        if event["timestamp"]["end"]:
            ts_end_raw = datetime.strptime(event["timestamp"]["end"]["date"], "%Y-%m-%d")
            if event["timestamp"]["end"]["time"]:
                ts_end_raw = ts_end_raw.replace(hour=int(event["timestamp"]["end"]["time"][:2]), minute=int(event["timestamp"]["end"]["time"][3:5]))

        # Convert to UTC for uniformity
        ts_start_utc = LOCAL_TZ.localize(ts_start_raw).astimezone(ZoneInfo("UTC"))
        ts_end_utc = LOCAL_TZ.localize(ts_end_raw).astimezone(ZoneInfo("UTC")) if ts_end_raw else None

        ev = Event(
            event["title"],
            begin=ts_start_utc,
            description=body.strip()
        )
        if ts_end_utc:
            ev.end = ts_end_utc
        if not event["timestamp"]["start"]["time"] and not event["timestamp"]["end"]:
            ev.make_all_day()
            # We need to change the start date to be un-localised!
            ev.begin = ts_start_raw
        if event["location"]:
            ev.location = event["location"]

        calendar.events.add(ev)

    ics_str = calendar.serialize()
    # Need to add a VTIMEZONE block for UTC, otherwise GCal freaks out
    ics_str_with_tz = ics_str.replace(
        "BEGIN:VCALENDAR\n",
        "BEGIN:VCALENDAR\n" + UTC_VTIMEZONE + "\n"
    )

    return ics_str_with_tz

if __name__ == "__main__":
    data = json.load(sys.stdin)

    ics_str = cal_to_ics(data)
    print(ics_str)
