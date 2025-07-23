#!/usr/bin/env python3
# Displays the given Polaris content.

from datetime import date, datetime
from typing import Literal, Tuple
from rich.console import Console, Group, group
from rich.markdown import Markdown, TextElement
from rich.padding import Padding
from rich.panel import Panel
from rich.table import Table
from rich.text import Text
from rich import print as rich_print
import re

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
    else:
        # NOTE: Target contexts are handled in `build_dashboards`
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
def cal_dashboard(items: list[dict], ty: Literal["events"] | Literal["daily_notes"]):
    """
    Returns a day-by-day dashboard over the given events or daily notes. These aren't
    combined anymore, but they are returned in a date-first dashboard, which is a different
    format from the other data types.
    """

    # For events and daily notes, the current date doesn't matter
    current_date = datetime.now().date()

    # Collect everything on a per-day basis
    dailies = {}
    for item in items:
        # Events get proper timestamps, daily notes just have a marker for which day they're on
        if ty == "events":
            date = datetime.strptime(item["timestamp"]["start"]["date"], "%Y-%m-%d").date()
        elif ty == "daily_notes":
            date = datetime.strptime(item["date"], "%Y-%m-%d").date()

        if date not in dailies:
            dailies[date] = []
        dailies[date].append(item)

    # Display the data one day at a time (guaranteed to be in order because order is
    # maintained from insertion and Polaris returns them in order)
    for idx, [date, items] in enumerate(dailies.items()):
        yield Text.from_markup(f"[bold underline]{date.strftime('%A, %B %d')}[/bold underline]")
        yield display_items(items, ty, current_date)

        if idx != len(dailies):
            yield Text()
    if not items:
        yield display_items([], ty, current_date)

def build_dashboards(json: dict, current_date: date):
    """
    Converts the given action item data from Polaris into a series of displayable objects. This
    will produce a context-aware panel for each section of data that's available in the output,
    doing logical things like combining the events and daily notes into day-by-day sections.

    Each value of the returned object will be a tuple of a position string and
    an actual renderable.
    """

    dashboards = []

    for view_name_str, view_data_map in json.items():
        view_name_parts = view_name_str.split("__", 1)
        view_name = view_name_parts[0]
        view_pos = view_name_parts[1] if len(view_name_parts) > 1 else None
        # The data is guaranteed be like `{"events": [..]}`
        view_data_type, view_data = next(iter(view_data_map.items()))
        if view_data_type == "events" or view_data_type == "daily_notes":
            displayed = cal_dashboard(view_data, view_data_type)
        elif view_data_type == "target_contexts":
            # For target contexts, we need to display an independent mini-dashboard
            # for every context
            context_minis = []
            for i, context_data in enumerate(sorted(view_data.items(), key=lambda x: x[0])):
                context, tasks = context_data
                context_minis.append((context, Group(
                    # Title for the context
                    Text.from_markup(f"[bold]{context.replace('_', ' ').capitalize()}[/bold]", justify="center"),
                    Text(),
                    display_items(tasks, "tasks", current_date),
                    Text() if i != len(view_data) - 1 else Group()
                )))

            context_dashboards = [t[1] for t in context_minis]
            displayed = Group(*context_dashboards)
        else:
            displayed = display_items(view_data, view_data_type, current_date)

        dashboards.append(PositionedDashboard(Panel(
            # Text.from_markup(f"[bold underline]{view_name}[/bold underline]", justify="center"),
            displayed, title=view_name
        ), view_name, view_pos))
        # TODO: What about when we *don't* have positions?

    return dashboards

def get_renderable_height(renderable):
    """
    Returns the height of the given renderable, computed with a virtual console.
    This will be relative to the width of the true console.
    """
    virtual_console = Console(width=Console().width, record=True, file=None)
    lines = virtual_console.render_lines(renderable, virtual_console.options, pad=False)

    return len(lines)

# This support column-spanning, which we don't generally support yet
POS_RE = re.compile(
    r"r:(?P<rs>\d+)(?:/(?P<re>\d+))?\s*;\s*c:(?P<cs>\d+)(?:/(?P<ce>\d+))?",
    flags=re.I,
)
def parse_pos(pos_str: str) -> Tuple[int, int, int, int]:
    """
    Parses the given position string (e.g. `r:1/3;c:2`) into a tuple of the starting
    row, ending row, starting column, and ending column. This will also perform
    elementary validation to ensure there aren't things like negative spans.

    All indices begin at zero in the output, but at one in the input!
    """
    m = POS_RE.fullmatch(pos_str.strip())
    if not m:
        raise ValueError(f"malformed position string: {pos_str!r}")

    row_start = int(m.group("rs")) - 1  # -1 => zero-based
    row_end = m.group("re")
    row_end = int(row_end) - 1 if row_end else row_start

    col_start = int(m.group("cs")) - 1
    col_end = m.group("ce")
    col_end = int(col_end) - 1 if col_end else col_start

    if row_start > row_end or col_start > col_end:
        raise ValueError(f"negative (backwards) span in {pos_str}")

    return row_start, row_end, col_start, col_end

class PositionedDashboard:
    """
    An all-in-one representation of a dashboard with a position in rows and columns that
    it will occupy in the final layout.
    """

    __slots__ = ("renderable", "row_start", "row_end", "col_start", "col_end", "height", "name")

    def __init__(self, renderable, name, pos: str):
        self.renderable = renderable
        self.name = name

        self.row_start, self.row_end, self.col_start, self.col_end = parse_pos(pos)
        self.height = get_renderable_height(renderable)

def build_layout(dashboards: list[PositionedDashboard]) -> Table:
    n_cols = max(d.col_end for d in dashboards) + 1

    # Accumulate the dashboards into column buckets (and ban column spanning)
    dashboards_by_col = {}
    for dashboard in dashboards:
        if dashboard.col_start != dashboard.col_end:
            raise ValueError(f"dashboard {dashboard.name} spans multiple columns, which is not (yet) supported")
        dashboards_by_col[dashboard.col_start] = dashboards_by_col.get(dashboard.col_start, []) + [dashboard]
    # Now sort them so we'll go through each column in row order (row spanning is handled implicitly here)
    dashboards_by_col = {k: sorted(v, key=lambda d: d.row_start) for k, v in dashboards_by_col.items()}

    root = Table.grid()
    cols = []
    for col_idx in range(n_cols):
        col = Table.grid()
        for row_dashboard in dashboards_by_col.get(col_idx, []):
            col.add_row(row_dashboard.renderable)
        cols.append(col)
    root.add_row(*cols)

    return root

if __name__ == "__main__":
    import json
    import sys

    current_date = datetime.strptime(sys.argv[1], "%Y-%m-%d") if len(sys.argv) > 1 else datetime.now().date()
    dashboards = build_dashboards(json.load(sys.stdin), current_date)

    layout = build_layout(dashboards)
    rich_print(layout)
