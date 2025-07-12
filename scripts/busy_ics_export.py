#!/usr/bin/env python
# A scheduling script that exports events from Polaris into a redacted ICS file saying
# when the user is busy versus not busy. This is designed to enable collaborative
# scheduling in a provider-agnostic manner, without leaking sensitive calendar
# information.

import re
import json
import sys
import os
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
        # Skip tasks and daily note events, they aren't actual time-blocks
        if event["ty"] != "event": continue

        ts_start = datetime.strptime(event["timestamp"]["start"]["date"], "%Y-%m-%d")
        ts_end = ts_start
        if event["timestamp"]["start"]["time"]:
            ts_start = ts_start.replace(hour=int(event["timestamp"]["start"]["time"][:2]), minute=int(event["timestamp"]["start"]["time"][3:5]))
        if event["timestamp"]["end"]:
            ts_end = datetime.strptime(event["timestamp"]["end"]["date"], "%Y-%m-%d")
            if event["timestamp"]["end"]["time"]:
                ts_end = ts_end.replace(hour=int(event["timestamp"]["end"]["time"][:2]), minute=int(event["timestamp"]["end"]["time"][3:5]))

        # Convert to UTC for uniformity
        ts_start = LOCAL_TZ.localize(ts_start).astimezone(ZoneInfo("UTC"))
        ts_end = LOCAL_TZ.localize(ts_end).astimezone(ZoneInfo("UTC")) if ts_end else None

        ev = Event(
            "🔒 BUSY",
            begin=ts_start,
            description="The details of this event have been redacted for privacy."
        )
        if ts_end:
            ev.end = ts_end
        if not event["timestamp"]["start"]["time"] and not event["timestamp"]["end"]:
            ev.make_all_day()

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

    ics_str = cal_to_ics(data["events"])
    print(ics_str)
