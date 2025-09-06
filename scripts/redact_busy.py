#!/usr/bin/env python
# A scheduling script that takes in an array of items from `items_to_list`, and redacts all
# identifying information from them, removing the title, body, and location, and redacting
# subtasks recursively, leaving only timestamps and IDs intact. This is intended to be used
# on calendar events to produce a "busy calendar" that can be shared with others without
# revealing sensitive information about the user's schedule.

import json
import sys

def redact_items(items):
    """
    Redacts the information on the given items, leaving only timestamps and IDs intact.
    """

    redacted = []
    # We're working with outputs of `items_to_list.py`
    for item in items:
        # Redact the title and description
        item["title"] = "🔒 BUSY"
        item["body"] = "The details of this event have been redacted for privacy."
        item["location"] = None
        item["subtasks"] = redact_items(item["subtasks"])

        redacted.append(item)

    return redacted

if __name__ == "__main__":
    data = json.load(sys.stdin)

    transformed = redact_items(data)
    json.dump(transformed, sys.stdout)
