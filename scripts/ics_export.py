#!/usr/bin/env python
# A scheduling script that takes an array of action items from stdin and prints an ICS file
# containing all action items with timestamps. This is intended to be composed with other scripts
# that filter those items.
#
# This can be told to disallow all-day events, which can be used to feed it *tasks* rather than
# events, and it will only export those with a clear start and end time.

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

def cal_to_ics(items, all_day_events):
    """
    Converts the given list of action items to an ICS calendar string.
    """

    calendar = Calendar()
    for item in items:
        ts_start_local = datetime.strptime(item["start"], "%Y-%m-%dT%H:%M:%S") if item["start"] else None
        ts_end_local = datetime.strptime(item["end"], "%Y-%m-%dT%H:%M:%S") if item["end"] else None
        if not ts_start_local:
            continue
        else:
            # Convert the timestamps to UTC for uniformity
            ts_start_utc = LOCAL_TZ.localize(ts_start_local).astimezone(ZoneInfo("UTC"))
            ts_end_utc = LOCAL_TZ.localize(ts_end_local).astimezone(ZoneInfo("UTC")) if ts_end_local else None

            ev = Event(
                item["title"],
                begin=ts_start_utc,
                # The body will already have all the information we need in it about people, etc.
                description=item["body"].strip()
            )
            if item["location"]:
                ev.location = item["location"]

            if ts_end_utc:
                ev.end = ts_end_utc

            # All-day events will arrive as events with no end datetime, and a start time of `00:00`
            if not ts_end_utc and ts_start_local.time() == datetime.min.time():
                if not all_day_events:
                    continue
                ev.make_all_day()
                # We need to change the start date to be un-localised!
                ev.begin = ts_start_local

            calendar.events.add(ev)

    ics_str = calendar.serialize()
    # Need to add a VTIMEZONE block for UTC, otherwise GCal freaks out
    ics_str_with_tz = ics_str.replace(
        "BEGIN:VCALENDAR\n",
        "BEGIN:VCALENDAR\n" + UTC_VTIMEZONE + "\n"
    )

    return ics_str_with_tz

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Convert action items to an ICS calendar.")
    parser.add_argument("-n", "--no-all-day", action="store_true", help="Disable all-day events")
    args = parser.parse_args()

    data = json.load(sys.stdin)

    ics_str = cal_to_ics(data, not args.no_all_day)
    print(ics_str)
