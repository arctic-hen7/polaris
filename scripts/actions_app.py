#!/usr/bin/env python
# Loads Polaris data from stdin and converts it to an HTML dashboard of actions, which
# allows the user to see contexts which need urgent focus, as well as filter tasks by
# relevant parameters. This will only operate on easy tasks, which are designed to be
# completed dynamically, rather than being scheduled for a particular day.

import os
import json
import sys

SELF_DIR = os.path.dirname(os.path.realpath(__file__))

def effort_to_numeric(effort):
    return {
        "minimal": 0,
        "low": 1,
        # These will never be touched for easy tasks, but left in for completeness
        "medium": 2,
        "high": 3,
        "total": 4,
    }[effort]

def priority_to_numeric(priority):
    return {
        "low": 0,
        "medium": 1,
        "high": 2,
        "important": 3,
    }[priority]

def format_tasks_for_app(tasks):
    """
    Takes in easy tasks from the Polaris output and reformats them for the actions app.
    This produces a very minimal format, in which contexts and people associated with
    tasks are referenced into an array.
    """

    # These will be referenced by indices, but we need fast lookup, so we have a hash
    # table to indices
    contexts = {}
    people = {}

    updated_tasks = []
    for task in tasks:
        task_contexts = []
        for ctx in task["contexts"]:
            if ctx not in contexts:
                contexts[ctx] = len(contexts)
            task_contexts.append(contexts[ctx])
        task_people = []
        for _, person in task["people"]:
            if person not in people:
                people[person] = len(people)
            task_people.append(people[person])

        updated_tasks.append([
            task["id"],
            task["title"],
            task["body"],
            task["scheduled"],
            task["deadline"],
            priority_to_numeric(task["priority"]),
            effort_to_numeric(task["effort"]),
            task_contexts,
            task_people,
        ])

    # These are guaranteed to come out in insertion order, which means the indices we saved
    # will be correct
    contexts_list = [k for k in contexts.keys()]
    people_list = [k for k in people.keys()]

    return [contexts_list, people_list, updated_tasks]

if __name__ == "__main__":
    data = json.load(sys.stdin)
    app_data = format_tasks_for_app(data["easy_tasks"])
    app_data_json = json.dumps(app_data)

    with open(f"{SELF_DIR}/actions_app/index.html", "r") as f:
        html = f.read()
    with open(f"{SELF_DIR}/actions_app/tailwind.css", "r") as f:
        css = f.read()
    with open(f"{SELF_DIR}/actions_app/index.js", "r") as f:
        js = f.read()

    final_html = html.replace("{{ styles }}", css).replace("{{ data }}", app_data_json).replace("{{ scripts }}", js)
    print(final_html)
