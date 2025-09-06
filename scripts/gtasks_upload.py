#!/usr/bin/env python
# Uploads the given list items to Google Tasks.

import os
import json
from typing import Optional
import requests
import jwt
import sys
from datetime import datetime, timedelta, UTC

GOOGLE_SCOPE = "https://www.googleapis.com/auth/tasks"

def get_access_token(service_account_info, scope, impersonate=None):
    """
    Uses the given service account details to get an ephemeral access token for the
    given scope, which allows actually interacting with the API.
    """

    issued_at = datetime.now(UTC)
    expiry = issued_at + timedelta(minutes=30)

    claims = {
        'iss': service_account_info['client_email'],
        'scope': scope,
        'aud': service_account_info['token_uri'],
        'exp': int(expiry.timestamp()),
        'iat': int(issued_at.timestamp()),
        'sub': impersonate or ""
    }

    header = {'alg': 'RS256', 'typ': 'JWT'}
    private_key = service_account_info['private_key']

    jwt_token = jwt.encode(
        payload=claims,
        key=private_key,
        algorithm='RS256',
        headers=header
    )

    response = requests.post(service_account_info['token_uri'], data={
        'grant_type': 'urn:ietf:params:oauth:grant-type:jwt-bearer',
        'assertion': jwt_token
    })
    if response.status_code == 200:
        return response.json()['access_token']
    else:
        raise Exception(f"Failed to get action items: {response.text}")

def get_task_list_id(list_name, token):
    """
    Gets the ID of the task list with the given name.
    """

    headers = {"Authorization": f"Bearer {token}"}
    response = requests.get(
        "https://tasks.googleapis.com/tasks/v1/users/@me/lists",
        headers=headers
    )
    if response.status_code != 200:
        raise Exception(f"Failed to get task lists: {response.text}")
    task_lists = response.json()["items"]
    for task_list in task_lists:
        if task_list["title"] == list_name:
            return task_list["id"]

    raise Exception(f"Task list '{list_name}' not found.")

def upload_item(item, token, task_list: str, parent_id: Optional[str]):
    """
    Converts the given list item into an object to be pushed to Google Tasks. This takes
    the task list to push to and the ID of the parent task.
    """

    headers = {"Authorization": f"Bearer {token}"}

    # Right now, the Google Tasks API can only tolerate a `due` *date*. We upload a time anyway,
    # but it gets ignored. Anything with *multiple* times is a definite no-no, so we ignore
    # anything with an end time as well.
    ts_start = datetime.strptime(item["start"], "%Y-%m-%dT%H:%M:%S") if item["start"] else None
    ts_end = datetime.strptime(item["end"], "%Y-%m-%dT%H:%M:%S") if item["end"] else None
    if ts_end:
        return

    task_obj = {
        "title": item["title"],
        "notes": item["body"],
        # Yes, we put this in UTC. No, Google does not read it as UTC. Does anything make sense? No, of course not.
        "due": datetime.strftime(ts_start, "%Y-%m-%dT%H:%M:%S+00:00") if ts_start else None,
        "status": "needsAction"
    }

    task_url = f"https://tasks.googleapis.com/tasks/v1/lists/{task_list}/tasks"
    if parent_id:
        task_url += f"?parent={parent_id}"

    response = requests.post(
        task_url,
        headers=headers,
        json=task_obj
    )
    if response.status_code != 200:
        print(f'Failed to push event: {response.text}')
        return

    # If this task had subtasks, upload them all under it
    if item["subtasks"]:
        task_id = response.json().get("id")
        for subtask in item["subtasks"]:
            upload_item(subtask, token, task_list, task_id)

def push_to_google_tasks(items, token, task_list):
    """
    Pushes the given list items to Google Calendar, using the given access token.
    """

    for item in items:
        upload_item(item, token, task_list, parent_id=None)

def upload_to_gtasks(events, email="env:GOOGLE_EMAIL", task_list="My Tasks", service_account_path="env:GOOGLE_CALENDAR_CREDS"):
    """
    Uploads the given list items to Google Tasks.

    The email and service_account parameters can either be provided as a raw email and path
    respectively, or as `env:ENV_VAR`s, which will load them from the environment automatically.
    """

    if service_account_path.startswith("env:"):
        env_var = service_account_path[4:]
        service_account_path = os.environ.get(env_var)
        if not service_account_path:
            raise Exception(f"No service account path in `{env_var}`")

    with open(service_account_path) as f:
        service_account_info = json.load(f)

    if email.startswith("env:"):
        env_var = email[4:]
        email = os.environ.get(env_var)
        if not email:
            raise Exception(f"No client email in `{env_var}`")

    token = get_access_token(service_account_info, GOOGLE_SCOPE, impersonate=email)
    task_list_id = get_task_list_id(task_list, token)
    push_to_google_tasks(events, token, task_list_id)

if __name__ == "__main__":
    data = json.load(sys.stdin)
    upload_to_gtasks(data, task_list=sys.argv[1] if len(sys.argv) > 1 else "My Tasks")
