#!/usr/bin/env python3
# Provides a quick shortcut to certain operations with Polaris.

import sys
import subprocess
import os
from datetime import datetime, timedelta

SELF_DIR = os.path.dirname(os.path.realpath(__file__))

def call_and_display(flags: list[str]):
    """
    Calls Polaris with the given flags and displays the output.
    """

    flags = ["polaris", "--json"] + flags
    polaris_proc = subprocess.run(flags, stdout=subprocess.PIPE, check=True)
    subprocess.run(["python", f"{SELF_DIR}/display.py"], input=polaris_proc.stdout, check=True)

current_date = datetime.now().strftime("%Y-%m-%d")
if len(sys.argv) == 1:
    # No arguments, give the user a rundown for the day
    call_and_display([current_date, current_date, "--events", "--daily-notes", "--target-contexts", current_date])
elif sys.argv[1] == "easy":
    # The user wants to filter on easy tasks
    filter_flags = sys.argv[2:]

    call_and_display([current_date, "--easy-tasks", *filter_flags])
elif sys.argv[1] == "hard":
    # The user wants to filter on easy tasks
    filter_flags = sys.argv[2:]

    call_and_display([current_date, "--hard-tasks", *filter_flags])
elif sys.argv[1] == "tasks":
    # The user wants to filter on easy tasks
    filter_flags = sys.argv[2:]

    call_and_display([current_date, "--tasks", *filter_flags])
elif sys.argv[1] == "cal":
    date_flags = sys.argv[2:]

    call_and_display([*date_flags, "--events", "--daily-notes"])
elif sys.argv[1] == "urgent":
    num_days_post = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    deadline = (datetime.now() + timedelta(days=num_days_post)).strftime("%Y-%m-%d")

    # Only show tasks with a deadline
    call_and_display([current_date, "--tasks", "--scheduled", current_date, "--deadline", deadline, "--force-match"])
elif sys.argv[1] == "upcoming":
    num_days_post = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    deadline = (datetime.now() + timedelta(days=num_days_post)).strftime("%Y-%m-%d")

    call_and_display([current_date, "--tasks", "--scheduled", current_date, "--deadline", deadline, "--force-match"])
elif sys.argv[1] == "day":
    date = sys.argv[2] if len(sys.argv) > 2 else datetime.now().strftime("%Y-%m-%d")
    deadline = (datetime.strptime(date, "%Y-%m-%d") + timedelta(days=3)).strftime("%Y-%m-%d")

    call_and_display([date, date, "--events", "--daily-notes", "--dates", "--tasks", "--scheduled", date, "--deadline", deadline, "--force-match"])
elif sys.argv[1] == "past":
    date = sys.argv[2] if len(sys.argv) > 2 else datetime.now().strftime("%Y-%m-%d")

    call_and_display([date, "--events", "--daily-notes", "--tickles", "--waits", "--dates", "--scheduled", date, "--deadline", date, "--force-match"])
elif sys.argv[1] == "week":
    start_date = sys.argv[2] if len(sys.argv) > 2 else datetime.now().strftime("%Y-%m-%d")
    end_date = (datetime.strptime(start_date, "%Y-%m-%d") + timedelta(days=7)).strftime("%Y-%m-%d")

    call_and_display([start_date, end_date, "--events", "--daily-notes", "--tasks", "--tickles", "--waits", "--dates", "--scheduled", end_date, "--deadline", end_date, "--force-match"])
else:
    print("Unknown command.")
    sys.exit(1)
