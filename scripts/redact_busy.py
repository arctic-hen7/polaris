#!/usr/bin/env python
# A scheduling script that modifies the given array of action items to only those that are
# events, redacting information about them. This is designed to be used in a calendar pipeline
# for producing a "busy calendar" which can be shared with others to let them know the user's
# availability without revealing sensitive information about the user's schedule.

import json
import sys

def redact_events(items):
    """
    Returns only those items from the input array which are events, and redacts their information.
    This will NOT change the ID of each item, as this is only meaningful with reference to data
    that would reveal the user's schedule anyway.
    """

    redacted = []
    for item in items:
        # Redact the title and description
        item["title"] = "🔒 BUSY"
        item["body"] = "The details of this event have been redacted for privacy."
        item["location"] = None
        item["people"] = []

        redacted.append(item)

    return redacted

if __name__ == "__main__":
    data = json.load(sys.stdin)

    transformed = redact_events(data)
    json.dump(transformed, sys.stdout)
