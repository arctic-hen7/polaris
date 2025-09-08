#!/usr/bin/env python
# Converts the given action items into a generic list format, putting most data into the body.
# This is used primarily for converting daily notes and tasks into a format that can be
# easily pushed to services like Google Tasks.

from datetime import datetime
import json
import sys

def date_to_human(date):
    """
    Converts the given machine-readable date into a human-readable one. This is much simpler than
    the general display logic, it only has a special case for the current day.
    """
    parsed = datetime.strptime(date, "%Y-%m-%dT%H:%M:%S")
    date_str = parsed.strftime("%A, %d %B %Y") if parsed.date() != datetime.now().date() else "Today"
    time_str = parsed.strftime("%H:%M")

    return f"{date_str} at {time_str}" if time_str else date_str

def process_ts(ts, earliest_date):
    """
    Processes the given *potential* timestamp and produces one where the dates are adjusted in
    accordance with the given earliest allowable date, in the form of a starting timestamp and
    and ending timestamp. These will be returned in the *local* timezone.
    """

    if not ts or not ts.get("start"):
        return None, None

    # Parse the start date, and time if it exists
    ts_start = datetime.strptime(ts["start"]["date"], "%Y-%m-%d")
    if ts["start"]["time"]:
        ts_start = ts_start.replace(hour=int(ts["start"]["time"][:2]), minute=int(ts["start"]["time"][3:5]))

    # And parse the end timestamp in the same way
    ts_end = None
    if ts["end"]:
        # If we only have a date and not a time, this will override a start time if one was present
        ts_end = datetime.strptime(ts["end"]["date"], "%Y-%m-%d")
        if ts["end"]["time"]:
            ts_end = ts_end.replace(hour=int(ts["end"]["time"][:2]), minute=int(ts["end"]["time"][3:5]))

    # Now overwrite those with `earliest_date` if either are before. This *will* turn datetimes into
    # datetimes that are actually just dates if necessary, which is the behaviour we want, because
    # otherwise tasks scheduled for previous days would be scheduled for the *same* time on a new
    # day, which is generally silly.
    # We check directly against the dates to avoid time comparison weirdness, and then use `00:00`
    # as the replacement time if needed (which is an all-day event).
    earliest_dt = datetime.combine(earliest_date, datetime.min.time())
    ts_start = ts_start if ts_start.date() >= earliest_date else earliest_dt
    ts_end = (ts_end if ts_end.date() >= earliest_date else earliest_dt) if ts_end else None

    return ts_start.isoformat(), (ts_end.isoformat() if ts_end else None)

def process_singleton_ts(dt_str, earliest_date):
    """
    Processes a single date into a starting timestamp, adjusting it in accordance with the
    given earliest allowable date. This will be returned in the *local* timezone.
    """

    dt = datetime.strptime(dt_str, "%Y-%m-%dT%H:%M:%S")
    earliest_dt = datetime.combine(earliest_date, datetime.min.time())
    dt = dt if dt >= earliest_dt else earliest_dt

    return dt.isoformat()

def transform_item(item, earliest_date):
    """
    Transforms the given item into a list item.
    """

    list_item = {
        "id": item["id"],
        "title": item["title"],
        "body": "",
        "location": item.get("location"), # Bit different, we keep this for calendar exports
        "subtasks": [],
        # These are formatted in the *local* timezone!
        "start": None,
        "end": None
    }

    body_parts = []
    # Add any relevant markers (contexts, priority, and people) to the body.
    # We check if there is a `contexts` key because, if it's empty, we'll handle it specially.
    if "contexts" in item:
        body_parts.append("**Contexts: " + ", ".join([c.replace("_", " ").title() for c in item["contexts"]]) + "**" if item["contexts"] else "**Contexts: (None)**")
    if item.get("priority"):
        body_parts.append(f"Priority: {item['priority'].capitalize()}")
    if item.get("people"):
        body_parts.append("People: " + ", ".join([name for _, name in item["people"]]))

    if item.get("date"):
        # If we've got a `date` property, we're working with something like a daily note, which
        # has very simple timestamp requirements
        item_dt = datetime.strptime(item["date"], "%Y-%m-%d")
        list_item["start"] = item_dt.isoformat() if item_dt.date() >= earliest_date else earliest_date.isoformat()
    else:
        # If we don't have a date, we *probably* have a timestamp
        if item.get("timestamp"):
            start, end = process_ts(item["timestamp"], earliest_date)
            list_item["start"] = start
            list_item["end"] = end

        # Now handle scheduled/deadline info. If we don't already have a timestamp, the
        # earlier of these will become it. We only check for the start because we can't
        # have an end timestamp without a starting one (`process_ts` ensures that).
        # We check scheduled first because Polaris guarantees it will come *before* the
        # deadline, and we should be optimistic in scheduling.
        if item.get("scheduled"):
            body_parts.append(f"Scheduled: {date_to_human(item['scheduled'])}")
            if not list_item["start"]:
                list_item["start"] = process_singleton_ts(item["scheduled"], earliest_date)

        if item.get("deadline"):
            body_parts.append(f"Deadline: {date_to_human(item['deadline'])}")
            if not list_item["start"]:
                list_item["start"] = process_singleton_ts(item["deadline"], earliest_date)

    # Finally, there will always be a body, even if it's empty
    if item["body"]:
        body_parts.append(f"\n{item['body'].strip()}")
    list_item["body"] = "\n".join(body_parts)
    list_item["body"] = list_item["body"].strip() if list_item["body"] else ""

    return list_item

def items_to_list(items, earliest_date):
    """
    Converts the given action items into a generic list format, putting most data into the body.
    This takes an earliest date, which any items which would be placed on dates before this will
    be instead placed onto. (This is intended to be used to schedule tasks based on their deadlines.)
    """

    transformed = []
    for type_name, type_data in items.items():
        if type_name == "target_contexts":
            # For target contexts, create a top-level task for each context, and then subtasks
            # within
            for ctx, ctx_data in type_data.items():
                ctx_item = {
                    "id": f"context_supertask_{ctx}",
                    "title": ctx.replace("_", " ").title(),
                    "body": "",
                    "location": None,
                    "start": None,
                    "end": None,
                    "subtasks": []
                }
                for item in ctx_data:
                    ctx_item["subtasks"].append(transform_item(item, earliest_date))

                transformed.append(ctx_item)
        else:
            # For everything else, just ignore types and convert everything
            for item in type_data:
                transformed.append(transform_item(item, earliest_date))

    return transformed

if __name__ == "__main__":
    earliest_date = datetime.strptime(sys.argv[1], "%Y-%m-%d").date() if len(sys.argv) > 1 else datetime.now().date()
    data = json.load(sys.stdin)

    transformed = items_to_list(data, earliest_date)
    json.dump(transformed, sys.stdout)
