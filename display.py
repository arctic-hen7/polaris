#!/usr/bin/env python3
# Displays the given Polaris content.

from datetime import date, datetime
from rich.console import Group, group
from rich.markdown import Markdown, TextElement
from rich.padding import Padding
from rich.panel import Panel
from rich.table import Table
from rich.text import Text
from rich import print as rich_print

class DisplayAdditional:
    """
    An "additional" bit of information in an item for display. These can all be formatted in the
    same way, in all-italics, in the form `<name>: [bold color]{<value>}[/bold color]`.

    If the value of the additional is `None`, it will not be displayed.
    """

    name: str
    value: str | datetime | None
    color: str

    def __init__(self, name, value, color):
        self.name = name
        self.value = value
        self.color = color

    def get_value_str(self, current_date: date) -> str | None:
        """
        Returns the value of the additional as a string, formatted for display.
        """

        if self.value is None:
            return None
        elif isinstance(self.value, datetime):
            return format_date(self.value.date(), self.value.strftime("%H:%M"), current_date)
        else:
            return self.value

class Timestamp:
    """
    A timestamp, stored in a Python-friendly representation of the original Orgish.
    """

    start: str | None = None
    end: str | None = None

    def __init__(self, ts_obj):
        self.start = ts_obj["start"]["time"]
        # Strip the seconds
        if self.start:
            self.start = self.start.removesuffix(":00")

        if ts_obj["end"]:
            self.end = ts_obj["end"]["time"]
            if self.end:
                self.end = self.end.removesuffix(":00")

class DisplayItem:
    """
    An item to be displayed.
    """

    title: str
    body: str | None
    additionals: list[DisplayAdditional]
    # These need special formatting
    time: Timestamp | None
    people: list[tuple[str, str]]

    def __init__(self, title, body, additionals = [], time = None, people = []):
        self.title = title
        self.body = body
        self.additionals = additionals
        self.time = time
        self.people = people

    @group()
    def display(self, current_date: date, is_last: bool):
        """
        Displays the item.
        """

        # Title and timestamp, if one exists
        if self.time:
            if self.time.start and self.time.end:
                time_str = f"from {self.time.start} to {self.time.end}"
            elif self.time.start:
                time_str = f"from {self.time.start}"
            elif self.time.end:
                time_str = f"until {self.time.end}"
            else:
                time_str = "all day"

            yield Text.from_markup(f"→ [bold]{self.title} [yellow]{time_str}[/yellow][/bold]")
        else:
            yield Text.from_markup(f"→ [bold]{self.title}[/bold]")

        # Additionals
        for additional in self.additionals:
            if additional.get_value_str(current_date):
                yield Text.from_markup(f"  {additional.name}: [bold {additional.color}]{additional.get_value_str(current_date)}[/bold {additional.color}]", style="italic")
        # People
        if len(self.people) > 0:
            yield Text.from_markup("  People needed:", style="italic")
            for _, name in self.people:
                yield Text.from_markup(f"    - [bold]{name}[/bold]", style="italic")

        # Body
        if self.body:
            Markdown.elements["heading"] = LeftJustifiedHeading
            # We can't pad in the string, so pad the whole thing
            yield Padding(
                Markdown(self.body, justify="left"),
                (1 if not self.body.startswith("- ") and not self.body.startswith("1. ") else 0, 0, 1 if not is_last else 0, 2)
            )
        elif not is_last:
            # If there is a body, the padding spaces us from the next item, if not, do that
            # manually
            yield Text("")

@group()
def display_items(items, ty, current_date: date):
    """
    Returns a Rich display of the given action items, with timestamps formatted relative to the
    given date.
    """

    for idx, item in enumerate(items):
        display_item = transform_item(item, ty)
        if not display_item: continue

        yield display_item.display(current_date, idx == len(items) - 1)

    if not items:
        yield Text.from_markup("[red italic]No items found.[/red italic]")

def transform_item(item, ty):
    """
    Transforms the given item according to our generic display format.
    """

    if ty == "events":
        # Date not shown, this is designed to be embedded in a date-specific container
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Location", value=item["location"], color="dodger_blue1"),
            ],
            time=Timestamp(item["timestamp"]),
            people=item["people"],
        )
    elif ty == "daily_notes":
        # Date not shown, this is designed to be embedded in a date-specific container
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[],
            time=None,
            people=[],
        )
    elif ty == "tickles":
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Appeared", value=datetime.strptime(item["date"], "%Y-%m-%d"), color="dodger_blue1")
            ],
            time=None,
            people=[],
        )
    elif ty == "person_dates":
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Date", value=datetime.strptime(item["date"], "%Y-%m-%d"), color="dodger_blue1"),
                DisplayAdditional(name="Person", value=item["person"][1], color="")
            ],
            time=None,
            people=[],
        )
    elif ty == "tasks":
        return DisplayItem(
            title=f"[NEXT] {item['title']}" if not item["can_start"] else item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Scheduled", value=datetime.strptime(item["scheduled"], "%Y-%m-%dT%H:%M:%S") if item["scheduled"] else None, color="dark_orange3"),
                DisplayAdditional(name="Deadline", value=datetime.strptime(item["deadline"], "%Y-%m-%dT%H:%M:%S") if item["deadline"] else None, color="red"),
                DisplayAdditional(name="Priority", value=item["priority"], color="green4"),
                DisplayAdditional(name="Context", value=", ".join(item["contexts"]) if item["contexts"] else None, color="dodger_blue1"),
                DisplayAdditional(name="Effort", value=item["effort"], color="blue"),
            ],
            time=None,
            people=item["people"],
        )
    elif ty == "projects":
        # TODO: Display subtasks and waiting items? Only if needed
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Scheduled", value=datetime.strptime(item["scheduled"], "%Y-%m-%dT%H:%M:%S") if item["scheduled"] else None, color="dark_orange3"),
                DisplayAdditional(name="Deadline", value=datetime.strptime(item["deadline"], "%Y-%m-%dT%H:%M:%S") if item["deadline"] else None, color="red"),
                DisplayAdditional(name="Priority", value=item["priority"], color="green4"),
            ],
            time=None,
            people=[],
        )
    elif ty == "waitings":
        return DisplayItem(
            title=item["title"],
            body=item["body"],
            additionals=[
                DisplayAdditional(name="Scheduled", value=datetime.strptime(item["scheduled"], "%Y-%m-%dT%H:%M:%S") if item["scheduled"] else None, color="dark_orange3"),
                DisplayAdditional(name="Deadline", value=datetime.strptime(item["deadline"], "%Y-%m-%dT%H:%M:%S") if item["deadline"] else None, color="red"),
                DisplayAdditional(name="Sent", value=datetime.strptime(item["sent"], "%Y-%m-%d") if item["sent"] else None, color="dodger_blue1"),
            ],
            time=None,
            people=[],
        )
    elif ty == "target_contexts":
        # Date isn't important here, these exist for whatever the requested period was
        return DisplayItem(
            title=item.capitalize(),
            body=None,
            additionals=[],
            time=None,
            people=[],
        )
    else:
        return None

def create_datetime(date_str, time_str=None):
    """
    Creates Python datetimes from Orgish time and date strings.
    """

    if time_str:
        return datetime.strptime(f"{date_str} {time_str}", "%Y-%m-%d %H:%M:%S")
    return datetime.strptime(date_str, "%Y-%m-%d")

def timestamp_to_datetime(timestamp):
    """
    Converts the given Orgish timestamp into a Python datetime.
    """

    ts_start = create_datetime(timestamp["start"]["date"], timestamp["start"]["time"])
    ts_end = create_datetime(timestamp["end"]["date"], timestamp["end"]["time"]) if timestamp["end"] else None
    return ts_start, ts_end

def format_date(date: date, time_str: str, current_date: date):
    """
    Formats the given date for human readability, relative to the given date, applying the
    given connective when it needs to give a specific weekday (e.g. on Wednesday, for Thursday).
    This connective should be chosen based on whatever comes before.
    """

    days_difference = (date - current_date).days
    weekdays = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]

    if days_difference == 0:
        day_str = "today"
    elif days_difference == 1:
        day_str = "tomorrow"
    elif days_difference == -1:
        day_str = "yesterday"
    elif 2 <= days_difference < 7:
        day_str = f"{weekdays[date.weekday()]}"
    elif -7 < days_difference < 0:
        day_str = f"last {weekdays[date.weekday()]}"
    elif 0 < days_difference < 14:
        day_str = f"next {weekdays[date.weekday()]}"
    else:
        day_str = f"{weekdays[date.weekday()]} {date.strftime('%d/%m/%Y')}"

    if time_str and time_str != "23:59" and time_str != "00:00":
        day_str += f" at {time_str}"

    return day_str

class LeftJustifiedHeading(TextElement):
    """
    A markdown heading, with styling designed for being embedded, rather than sticking out in the
    centre of the terminal.
    """

    @classmethod
    def create(cls, markdown, node) -> "LeftJustifiedHeading":
        heading = cls(node.level)
        return heading

    def on_enter(self, context):
        self.text = Text()
        context.enter_style(self.style_name)

    def __init__(self, level: int):
        self.level = level
        super().__init__()

    def __rich_console__(
        self, console, options
    ):
        text = self.text
        text.justify = "left"
        yield Text(f"{' ' * (self.level - 1)}→ {text}")

@group()
def cal_dashboard(events: list[dict] | None, notes: list[dict] | None):
    """
    Returns a day-by-day dashboard over the given events and notes, if either are present.
    """

    # For events and daily notes, the current date doesn't matter
    current_date = datetime.now().date()

    # Collect everything on a per-day basis
    dailies = {}
    for event in events or []:
        event_date = datetime.strptime(event["timestamp"]["start"]["date"], "%Y-%m-%d").date()
        if event_date not in dailies:
            dailies[event_date] = {"events": [], "notes": []}
        dailies[event_date]["events"].append(event)
    for note in notes or []:
        note_date = datetime.strptime(note["date"], "%Y-%m-%d").date()
        if note_date not in dailies:
            dailies[note_date] = {"events": [], "notes": []}
        dailies[note_date]["notes"].append(note)

    # Display the data one day at a time (guaranteed to be in order because order is
    # maintained from insertion and Polaris returns them in order)
    for date, day in dailies.items():
        yield Text.from_markup(f"[bold underline]{date.strftime('%A, %B %d')}[/bold underline]")

        # Pad the daily notes under a heading
        if day["notes"]:
            yield Text.from_markup("[bold italic]Daily Notes:[/bold italic]")
        yield Padding(display_items(day["notes"], "daily_notes", current_date), (0, 0, 0, 2))
        yield Text()

        # And show the events as the primary component
        yield display_items(day["events"], "events", current_date)

def build_dashboards(json: dict, current_date: date):
    """
    Converts the given action item data from Polaris into a series of displayable objects. This
    will produce a context-aware panel for each section of data that's available in the output,
    doing logical things like combining the events and daily notes into day-by-day sections.
    """

    dashboards = {}
    # Events and daily notes get combined into a special day-by-day dashboard
    if json["events"] or json["daily_notes"]:
        dashboards["cal"] = cal_dashboard(json["events"], json["daily_notes"])

    if json["tickles"]:
        dashboards["tickles"] = Group(
            Text.from_markup("[bold underline]Tickles[/bold underline]", justify="center"),
            display_items(json["tickles"], "tickles", current_date)
        )

    if json["person_dates"]:
        dashboards["person_dates"] = Group(
            Text.from_markup("[bold underline]Important Dates[/bold underline]", justify="center"),
            display_items(json["person_dates"], "person_dates", current_date)
        )

    if json["hard_tasks"]:
        dashboards["hard_tasks"] = Group(
            Text.from_markup("[bold underline]Hard Tasks[/bold underline]", justify="center"),
            display_items(json["hard_tasks"], "tasks", current_date)
        )

    if json["easy_tasks"]:
        dashboards["easy_tasks"] = Group(
            Text.from_markup("[bold underline]Easy Tasks[/bold underline]", justify="center"),
            display_items(json["easy_tasks"], "tasks", current_date)
        )

    if json["projects"]:
        dashboards["projects"] = Group(
            Text.from_markup("[bold underline]Projects[/bold underline]", justify="center"),
            display_items(json["projects"], "projects", current_date)
        )

    if json["waitings"]:
        dashboards["waitings"] = Group(
            Text.from_markup("[bold underline]Waiting Items[/bold underline]", justify="center"),
            display_items(json["waitings"], "waitings", current_date)
        )

    # crunch_points = None
    # if json["crunch_points"]:
    #     crunch_points = Group(
    #         Text.from_markup("[bold underline]crunch_points[/bold underline]", justify="center"),
    #         display_items(json["crunch_points"], "crunch_points", current_date)
    #     )

    if json["target_contexts"]:
        dashboards["target_contexts"] = Group(
            Text.from_markup("[bold underline]Urgent Contexts[/bold underline]", justify="center"),
            display_items(json["target_contexts"], "target_contexts", current_date)
        )

    return dashboards

if __name__ == "__main__":
    import json
    import sys

    # Known alignments, specified as rows of columns (e.g. `[["cal", "dates"]]` -> cal above
    # dates in a single column)
    ALIGNMENTS = {
        # Past dashboard
        "cal,dates,tickles,waiting": [["cal", "dates"], ["tickles", "waiting"]],
        # Day dashboard
        "cal,dates,easy_tasks,hard_tasks": [["cal", "dates"], ["hard_tasks", "easy_tasks"]],
        # Week dashboard
        "cal,dates,easy_tasks,hard_tasks,tickles,waiting": [["cal", "dates", "tickles"], ["waiting", "hard_tasks", "easy_tasks"]]
    }

    current_date = datetime.strptime(sys.argv[1], "%Y-%m-%d") if len(sys.argv) > 1 else datetime.now().date()
    dashboards = build_dashboards(json.load(sys.stdin), current_date)

    # Get a string like `cal,tasks` saying which dashboards are available (sorted for determinism)
    available_dashboards = ",".join(sorted(dashboards.keys()))
    # Use that to figure out the alignment we should take, or just print them in order of
    # insertion
    if available_dashboards in ALIGNMENTS:
        view = Table.grid()
        columns = []
        for column in ALIGNMENTS[available_dashboards]:
            view_col = Table.grid()
            for row in column:
                view_col.add_row(Panel(dashboards[row]))
            columns.append(view_col)
        view.add_row(*columns)

        rich_print(view)
    else:
        for dashboard in dashboards.values():
            rich_print(dashboard)
