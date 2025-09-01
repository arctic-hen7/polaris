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

def transform_item(item, earliest_date):
    """
    Transforms the given item into a list item.
    """

    list_item = {
        "id": item["id"],
        "title": item["title"],
        "body": "",
        "timestamp": None,
        "subtasks": []
    }

    if "date" in item:
        # Daily notes are quite easy to handle
        item_date = datetime.strptime(item["date"], "%Y-%m-%d").date()
        list_item["date"] = {
            "start": {
                "date": item_date.isoformat() if item_date >= earliest_date else earliest_date,
                "time": None
            },
            "end": None,
        }
        list_item["body"] = item["body"]
    else:
        # For tasks, we have some more things to insert into the body
        body_parts = []
        body_parts.append("**Contexts: " + ", ".join([c.replace("_", " ").title() for c in item["contexts"]]) + "**" if item.get("contexts") else "**Contexts: (None)**")
        body_parts.append(f"Priority: {item['priority'].capitalize()}")

        # Take the date from the start of the timestamp
        if item["timestamp"]:
            list_item["timestamp"] = item["timestamp"]
            # Replace dates with `earliest_date` if needed
            start_date = datetime.strptime(item["timestamp"]["start"]["date"], "%Y-%m-%d").date()
            if start_date < earliest_date:
                list_item["timestamp"]["start"]["date"] = earliest_date.isoformat()
            if item["timestamp"]["end"] and datetime.strptime(item["timestamp"]["end"]["date"], "%Y-%m-%d").date() < earliest_date:
                list_item["timestamp"]["end"]["date"] = earliest_date.isoformat()

        if item["scheduled"]:
            body_parts.append(f"Scheduled: {date_to_human(item['scheduled'])}")
        if item["deadline"]:
            body_parts.append(f"Deadline: {date_to_human(item['deadline'])}")
            # Also schedule based on the deadline if we didn't get a timestamp
            if not list_item["timestamp"]:
                deadline = datetime.strptime(item["deadline"], "%Y-%m-%dT%H:%M:%S").date()
                list_item["timestamp"] = {
                    "start": {
                        "date": deadline.isoformat() if deadline >= earliest_date else earliest_date.isoformat(),
                        "time": None
                    },
                    "end": None,
                }

        if item.get("people"):
            body_parts.append("People: " + ", ".join([name for _, name in item["people"]]))

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
                    # Subtasks might have different timestamps
                    "timestamp": None,
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
