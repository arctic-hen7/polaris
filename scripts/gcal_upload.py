#!/usr/bin/env python
# Uploads the given calendar entries to Google Calendar. Unlike exporting a static ICS file, this
# allows the entries to be modified live, enabling the greater dynamism needed in real-world
# settings. Typically, I'll use this script for the current day, and then use the ICS export for
# the next forseeable period.

import os
import json
import requests
import jwt
import sys
from datetime import datetime, timedelta, UTC

GOOGLE_SCOPE = "https://www.googleapis.com/auth/calendar"

def get_access_token(service_account_info, scope, impersonate=None):
    """
    Uses the given service account details to get an ephemeral access token for the
    given scope, which allows actually interacting with the calendar.
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

def push_to_google_calendar(items, token, calendar, all_day_events):
    """
    Pushes the given calendar entries to Google Calendar, using the given access token.
    """

    headers = {"Authorization": f"Bearer {token}"}
    local_tz = datetime.now().astimezone().tzinfo

    for item in items:
        ts_start_local = datetime.strptime(item["start"], "%Y-%m-%dT%H:%M:%S") if item["start"] else None
        ts_end_local = datetime.strptime(item["end"], "%Y-%m-%dT%H:%M:%S") if item["end"] else None
        if not ts_start_local:
            return

        # Localise the timestamps first (GCal needs this)
        ts_start = ts_start_local.replace(tzinfo=local_tz)
        ts_end = ts_end_local.replace(tzinfo=local_tz) if ts_end_local else None

        # Detect all-day events
        if not ts_end and ts_start_local.time() == datetime.min.time():
            if not all_day_events:
                continue
            start = {"date": ts_start.date().isoformat()}
            end = {"date": ts_start.date().isoformat()}
        else:
            start = {"dateTime": ts_start.isoformat()}
            end = {"dateTime": ts_end.isoformat()} if ts_end else None

        event = {
            "summary": item["title"],
            "description": item["body"],
            "location": item["location"],
            "start": start,
            "end": end
        }

        response = requests.post(
            f"https://www.googleapis.com/calendar/v3/calendars/{calendar}/events",
            headers=headers,
            json=event
        )
        if response.status_code != 200:
            print(f'Failed to push event: {response.text}')

def upload_to_gcal(events, all_day_events, email="env:GOOGLE_EMAIL", calendar="primary", service_account_path="env:GOOGLE_CALENDAR_CREDS"):
    """
    Uploads the given calendar items to Google Calendar

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
    push_to_google_calendar(events, token, calendar, all_day_events)

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Upload action items to Google Calendar.")
    parser.add_argument("-n", "--no-all-day", action="store_true", help="Disable all-day events")
    args = parser.parse_args()

    data = json.load(sys.stdin)
    upload_to_gcal(data, not args.no_all_day)
