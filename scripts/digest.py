#!/usr/bin/env python
# Calls Polaris to get information about the given date for producing a Markdown digest
# of the events and tasks scheduled for the day, daily notes, and any easy tasks that
# need to be completed by then. Goals are also included (this can be disabled to avoid
# calling the Polaris goal parsing system with `--no-goals`).

import subprocess
import json

def get_data(flags: list[str]):
    """
    Calls Polaris with the given flags and returns the JSON output.
    """

    flags = ["polaris", "--json"] + flags

    polaris_proc = subprocess.run(flags, stdout=subprocess.PIPE, check=True)
    return json.loads(polaris_proc.stdout)

def cal_digest(data):
    """
    Produces a Markdown digest of events from the given data.
    """

    cal_md = "## Events\n\n"
    for event in data["events"]:
        # We know all events we have start and end on this day, so we can ignore the date
        ts = event["timestamp"]
        if ts["start"] and ts["start"]["time"] and ts["end"] and ts["end"]["time"]:
            time_str = f"from **{ts['start']['time'].removesuffix(':00')}** to **{ts['end']['time'].removesuffix(':00')}**"
        elif ts["start"] and ts["start"]["time"]:
            time_str = f"from **{ts['start']['time'].removesuffix(':00')}**"
        elif ts["end"] and ts["end"]["time"]:
            time_str = f"until **{ts['end']['time'].removesuffix(':00')}**"
        else:
            time_str = "**all day**"

        if event["location"]:
            loc_str = f" at *{event['location']}*"
        else:
            loc_str = ""

        event["title"] = f"({event['ty'].upper()}) {event['title']}"

        cal_md += f"- {event['title']} ({time_str}{loc_str})\n"
    if not data["events"]:
        cal_md += "*No events.*"

    return cal_md.strip()

def daily_notes_digest(data):
    """
    Produces a Markdown digest of daily notes from the given data.
    """

    daily_notes_md = "## Daily Notes\n\n"
    for note in data["daily_notes"]:
        daily_notes_md += f"- {note['title']}\n"
    if not data["daily_notes"]:
        daily_notes_md += "*No daily notes.*"

    return daily_notes_md.strip()

def urgent_tasks_digest(data):
    """
    Produces a Markdown digest of the urgent tasks that need to be completed by some date from
    the given data.
    """

    easy_tasks_md = "## Urgent Tasks\n\n"
    for task in data["easy_tasks"]:
        easy_tasks_md += f"- {task['title']}\n"
    if not data["easy_tasks"]:
        easy_tasks_md += "*No urgent tasks.*"

    return easy_tasks_md.strip()

if __name__ == "__main__":
    import argparse
    from datetime import datetime, timedelta

    parser = argparse.ArgumentParser(description="Return a digest for the given date.")
    parser.add_argument("date", type=str, nargs="?", help="The date to return a digest for")
    parser.add_argument("--no-goals", action="store_true", help="Don't query for or output goals in the digest")

    args = parser.parse_args()
    if args.date == "tmrw" or args.date == "tomorrow":
        date = (datetime.now() + timedelta(days=1)).strftime("%Y-%m-%d")
    elif not args.date:
        date = datetime.now().strftime("%Y-%m-%d")
    else:
        date = args.date

    data = get_data([date, date, "--events", "--daily-notes", "--easy-tasks", "--scheduled", date, "--deadline", date, "--force-deadline"])
    digest = f"# Digest for {datetime.strptime(date, '%Y-%m-%d').strftime('%A, %d %B %Y')}\n\n{cal_digest(data)}\n\n{daily_notes_digest(data)}\n\n{urgent_tasks_digest(data)}"

    print(digest)
