use crate::{
    extractors::{DailyNote, Event, PersonDate, Project, Task, Tickle, Waiting},
    parse::Priority,
};
use anyhow::{bail, Error};
use chrono::{NaiveDate, NaiveDateTime};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

/// A single "view" over data. Polaris will filter data according to this view, which can contain
/// exactly one type of item (e.g. events, tasks, etc.) and a set of filters to apply to that type.
#[derive(Deserialize, Subcommand, Clone, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
#[command(rename_all = "lowercase")]
pub enum View {
    /// Items with specific timestamps, usually calendar events or tasks scheduled for a specific
    /// date (and potentially time).
    Events(EventsFilter),
    /// Items with the `NOTE` keyword, which have a date associated with them. These are designed
    /// to record things to remember for a particular day.
    DailyNotes(DailyNotesFilter),
    /// Items under a `tickles` parent tag with an associated date. These are intended to be
    /// reminders of things to re-examine on a specific date (typically items that go through the
    /// inbox and should be reviewed on a later date).
    Tickles(TicklesFilter),
    /// Items under a `person_dates` parent tag with an associated date. These are used to record
    /// birthdays and similar. They will be shown in a date range.
    Dates(DatesFilter),
    /// Items with the `WAIT` keyword, used to track things the user is waiting on from others.
    /// These are typically organised with scheduled and deadline dates.
    Waits(WaitsFilter),
    /// Items with the `PROJ` keyword, used to track groups of tasks with overarching
    /// scheduled/deadline dates.
    Projects(ProjectsFilter),
    /// Items with the `TODO` or `NEXT` keyword, which indicate tasks that the user should
    /// complete. These are organised with a combination of scheduled/deadline dates, "contexts"
    /// (which might represent the place the task can be completed in, something about the
    /// conditions under which it can be completed, etc.), priorities, and people involved.
    ///
    /// Tasks with the `TODO` keyword are considered immediately actionable, whereas those with the
    /// `NEXT` keyword are non-actionable (typically after some other actionable ones are
    /// completed, within the same project).
    Tasks(TasksFilter),
    /// If specified, a list of contexts that need to be "entered" to complete all tasks by the
    /// given date will be produced, including the details of those tasks which need to be
    /// completed. Formally, this will go through all tasks with deadlines on or before the given
    /// date, and will produce the list of these tasks, organised by context (if a task has
    /// multiple contexts, it will appear in each context's list).
    #[command(name = "target_contexts")]
    TargetContexts(TargetContextsFilter),
}
impl View {
    /// Validates the order of dates passed to this view. For instance, if this is a
    /// [`View::Events`], this will make sure the `until` date is after the `from` date. This will
    /// similarly ensure deadline dates are after scheduled dates.
    ///
    /// This will also return the *last* date in the view, which provides a minimum point to expand
    /// all repeating timestamps up until (usually, a buffer will be added to this to account for
    /// things like long notification times on person dates). Note that not all views will involve
    /// a filter on dates, so this may return [`None`] in that case.
    pub fn validate(&self) -> Result<Option<NaiveDate>, Error> {
        match &self {
            Self::Events(EventsFilter { from, until }) => {
                if from.is_some_and(|f| *until < f) {
                    bail!("`until` date must be after `from` date");
                }
                Ok(Some(*until))
            }
            Self::DailyNotes(DailyNotesFilter { from, until }) => {
                if from.is_some_and(|f| *until < f) {
                    bail!("`until` date must be after `from` date");
                }
                Ok(Some(*until))
            }
            Self::Tickles(TicklesFilter { until }) => Ok(Some(*until)),
            Self::Dates(DatesFilter { until }) => Ok(Some(*until)),
            Self::Waits(WaitsFilter {
                scheduled,
                deadline,
                planning_match: _,
            }) => {
                if deadline.is_some_and(|d| scheduled.is_some_and(|s| d < s)) {
                    bail!("`deadline` date must be after `scheduled` date");
                }
                Ok(scheduled.or(*deadline))
            }
            Self::Projects(ProjectsFilter {
                scheduled,
                deadline,
                planning_match: _,
            }) => {
                if deadline.is_some_and(|d| scheduled.is_some_and(|s| d < s)) {
                    bail!("`deadline` date must be after `scheduled` date");
                }
                Ok(scheduled.or(*deadline))
            }
            Self::Tasks(TasksFilter {
                scheduled,
                deadline,
                planning_match: _,
                next_tasks: _,
                contexts: _,
                min_priority: _,
                max_priority: _,
                people: _,
            }) => {
                if deadline.is_some_and(|d| scheduled.is_some_and(|s| d < s)) {
                    bail!("`deadline` date must be after `scheduled` date");
                }
                Ok(scheduled.or(*deadline))
            }
            Self::TargetContexts(TargetContextsFilter { until }) => Ok(Some(*until)),
        }
    }
}

#[derive(Parser, Debug, Clone, Deserialize)]
pub struct EventsFilter {
    /// The date on which to start showing items from (inclusive). If this is `None`, all
    /// items before `until` will be shown.
    #[arg(short, long)]
    from: Option<NaiveDate>,
    /// The date at which to stop showing items (inclusive).
    #[arg(short, long)]
    until: NaiveDate,
}
impl EventsFilter {
    /// Checks if the given event matches this filter or not.
    pub fn matches(&self, ev: &Event) -> bool {
        ev.timestamp.start.date <= self.until
            && self.from.is_none_or(|from| {
                ev.timestamp
                    .end
                    .as_ref()
                    .unwrap_or(&ev.timestamp.start)
                    .date
                    >= from
            })
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct DailyNotesFilter {
    /// The date on which to start showing items from (inclusive). If this is `None`, all
    /// items before `until` will be shown.
    #[arg(short, long)]
    from: Option<NaiveDate>,
    /// The date at which to stop showing items (inclusive).
    #[arg(short, long)]
    until: NaiveDate,
}
impl DailyNotesFilter {
    pub fn matches(&self, dn: &DailyNote) -> bool {
        dn.date <= self.until && self.from.is_none_or(|from| dn.date >= from)
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct TicklesFilter {
    /// The date at which to stop showing tickles. All tickles on or before this date will be
    /// shown (as old ones that haven't been handled yet need to be under the conceptual model
    /// for tickles).
    #[arg(short, long)]
    until: NaiveDate,
}
impl TicklesFilter {
    pub fn matches(&self, t: &Tickle) -> bool {
        t.date <= self.until
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct DatesFilter {
    /// The reference date at which to stop showing important dates. Dates have *notify dates*
    /// associated with them, which are typically something like a week, month, etc. before the
    /// actual date. This `until` date will be a filter for these notification dates, *not* the
    /// main dates themselves!
    ///
    /// E.g. if there's a birthday on the 10th of January with a 1-week notification date,
    /// this will show it if the `until` date is on or after the 3rd of January.
    #[arg(short, long)]
    until: NaiveDate,
}
impl DatesFilter {
    pub fn matches(&self, d: &PersonDate) -> bool {
        d.notify_date <= self.until
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct WaitsFilter {
    /// A scheduled date on an item indicates when it should first be surfaced to the user, and
    /// this will show all items that should be surfaced on or before this date. If not
    /// present, items won't be filtered by their scheduled date.
    #[arg(short, long)]
    scheduled: Option<NaiveDate>,
    /// A deadline date on an item indicates when it should be completed by. This will show all
    /// items that should be completed on or before this date. If not present, items won't be
    /// filtered by their deadline dates.
    #[arg(short, long)]
    deadline: Option<NaiveDate>,
    /// The mode of matching to use for scheduled and deadline dates. If you aren't specifying
    /// filters for these, this should be [`PlanningMatchType::All`], otherwise you'll filter
    /// to only items that have a scheduled/deadline date, without filtering on that date
    /// itself (which may be desired, but usually isn't).
    #[arg(short = 'm', long = "match", default_value = "all")]
    #[serde(default)]
    planning_match: PlanningMatchType,
}
impl WaitsFilter {
    pub fn matches(&self, w: &Waiting) -> bool {
        meets_dt(
            w.scheduled,
            self.scheduled,
            // We should accept items that don't have a `scheduled` date, *unless* we explicitly
            // require `scheduled` dates
            self.planning_match == PlanningMatchType::ScheduledOnly,
        ) && meets_dt(
            w.deadline,
            self.deadline,
            self.planning_match == PlanningMatchType::DeadlineOnly,
        ) && (self.planning_match != PlanningMatchType::ScheduledOrDeadline
            || w.scheduled.is_some()
            || w.deadline.is_some())
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct ProjectsFilter {
    /// A scheduled date on an item indicates when it should first be surfaced to the user, and
    /// this will show all items that should be surfaced on or before this date. If not
    /// present, items won't be filtered by their scheduled date.
    #[arg(short, long)]
    scheduled: Option<NaiveDate>,
    /// A deadline date on an item indicates when it should be completed by. This will show all
    /// items that should be completed on or before this date. If not present, items won't be
    /// filtered by their deadline dates.
    #[arg(short, long)]
    deadline: Option<NaiveDate>,
    /// The mode of matching to use for scheduled and deadline dates. If you aren't specifying
    /// filters for these, this should be [`PlanningMatchType::All`], otherwise you'll filter
    /// to only items that have a scheduled/deadline date, without filtering on that date
    /// itself (which may be desired, but usually isn't).
    #[arg(short = 'm', long = "match", default_value = "all")]
    #[serde(default)]
    planning_match: PlanningMatchType,
}
impl ProjectsFilter {
    pub fn matches(&self, p: &Project) -> bool {
        meets_dt(
            p.scheduled,
            self.scheduled,
            self.planning_match == PlanningMatchType::ScheduledOnly,
        ) && meets_dt(
            p.deadline,
            self.deadline,
            self.planning_match == PlanningMatchType::DeadlineOnly,
        ) && (self.planning_match != PlanningMatchType::ScheduledOrDeadline
            || p.scheduled.is_some()
            || p.deadline.is_some())
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct TasksFilter {
    /// A scheduled date on an item indicates when it should first be surfaced to the user, and
    /// this will show all items that should be surfaced on or before this date. If not
    /// present, items won't be filtered by their scheduled date.
    #[arg(short, long)]
    scheduled: Option<NaiveDate>,
    /// A deadline date on an item indicates when it should be completed by. This will show all
    /// items that should be completed on or before this date. If not present, items won't be
    /// filtered by their deadline dates.
    #[arg(short, long)]
    deadline: Option<NaiveDate>,
    /// The mode of matching to use for scheduled and deadline dates. If you aren't specifying
    /// filters for these, this should be [`PlanningMatchType::All`], otherwise you'll filter
    /// to only items that have a scheduled/deadline date, without filtering on that date
    /// itself (which may be desired, but usually isn't).
    #[arg(short = 'm', long = "match", default_value = "all")]
    #[serde(default)]
    planning_match: PlanningMatchType,
    /// Whether or not to show non-actionable tasks with the `NEXT` keyword.
    #[arg(short, long)]
    #[serde(default)]
    next_tasks: bool,
    /// The contexts we have "available". Specifying these will filter to only tasks which have
    /// all their required contexts present in this list (tasks with no contexts will not be
    /// shown unless an empty list is provided). If this is not specified, tasks will not be
    /// filtered by their contexts.
    #[arg(short, long)]
    contexts: Option<Vec<String>>,
    /// The minimum priority of tasks to show.
    #[arg(long)]
    min_priority: Option<Priority>,
    /// The maximum priority of tasks to show.
    #[arg(long)]
    max_priority: Option<Priority>,
    /// A list of people to filter tasks by, showing only those tasks which have all their
    /// required people present in this list (tasks with no people will not be shown unless an
    /// empty list is provided). If this is not specified, tasks will not be filtered by their
    /// required people.
    ///
    /// Note that elements in this list should be the names of people, not their IDs as
    /// Starling nodes.
    #[arg(short, long)]
    people: Option<Vec<String>>,
}
impl TasksFilter {
    pub fn matches(&self, t: &Task) -> bool {
        // -- Usual scheduled/deadline filtering --
        meets_dt(
            t.scheduled,
            self.scheduled,
            self.planning_match == PlanningMatchType::ScheduledOnly,
        ) && meets_dt(
            t.deadline,
            self.deadline,
            self.planning_match == PlanningMatchType::DeadlineOnly,
        ) && (self.planning_match != PlanningMatchType::ScheduledOrDeadline
            || t.scheduled.is_some()
            || t.deadline.is_some()) &&
        // -- The rest --
        // Either we allow non-actionable tasks, or this task must be actionable
        (self.next_tasks || t.can_start) &&
        // Either we aren't filtering by contexts, or we're showing only tasks with no contexts, or
        // we're showing tasks with contexts where we have all their contexts
        (self.contexts.is_none() || (self.contexts.as_ref().is_some_and(|c| c.is_empty()) && t.contexts.is_empty()) || (t.contexts.iter().all(|c| {
            self.contexts.as_ref().unwrap().contains(c)
        }) && !t.contexts.is_empty())) &&
        // Either we aren't filtering by priorities, or the task's priority is within the range
        self.min_priority.is_none_or(|min_p| t.priority >= min_p) &&
         self.max_priority.is_none_or(|max_p| t.priority <= max_p) &&
        // Filtering by people is the same as filtering by contexts
        (self.people.is_none() || (self.people.as_ref().is_some_and(|p| p.is_empty()) && t.people.is_empty()) || (t.people.iter().all(|(_id, p)| {
            self.people.as_ref().unwrap().contains(p)
        }) && !t.people.is_empty()))
    }

    /// Creates a new filter for tasks that are relevant to determining the target contexts that
    /// meet the given [`TargetContextsFilter`].
    pub fn for_target_contexts(filter: &TargetContextsFilter) -> Self {
        Self {
            scheduled: None,
            deadline: Some(filter.until),
            planning_match: PlanningMatchType::DeadlineOnly,
            next_tasks: true,
            contexts: None,
            min_priority: None,
            max_priority: None,
            people: None,
        }
    }
}
#[derive(Parser, Debug, Clone, Deserialize)]
pub struct TargetContextsFilter {
    /// The date by which all tasks should be completed. This will be used to filter tasks by
    /// their deadlines, and the contexts for those tasks will be produced.
    #[arg(short, long)]
    until: NaiveDate,
}

/// Determines whether or not a date on an item meets an imposed cutoff (e.g. its deadline is
/// before the cutoff).
///
/// If the imposed date is not present, this will return true. If it is present and the item has a
/// date as well, it will return true if the item's date is before or on the cutoff. If the item
/// does *not* have a date, it will return true if `force_imposed` is false, and false if it is
/// true. That is, when `force_imposed` is set, items without dates will not be allowed, whereas
/// when it's not, they will be, provided they meet the cutoff.
fn meets_dt(
    item_dt: Option<NaiveDateTime>,
    imposed_date: Option<NaiveDate>,
    force_imposed: bool,
) -> bool {
    imposed_date.is_none()
        || item_dt.map_or(!force_imposed, |item_dt| {
            item_dt.date() <= imposed_date.unwrap()
        })
}

/// The type of matching to be performed when filtering by scheduled/deadline dates.
#[derive(Deserialize, ValueEnum, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "lowercase")]
enum PlanningMatchType {
    /// Show all items not excluded by the scheduled/deadline criteria. This will show items that
    /// have no scheduled/deadline dates as well as those that do.
    All,
    /// Show only items that have a scheduled date that matches the given criterion. Items which
    /// would match because they have no scheduled date will not be shown.
    ScheduledOnly,
    /// Show only items that have a deadline date that matches the given criterion. Items which
    /// would match because they have no deadline will not be shown.
    DeadlineOnly,
    /// Show only items that have either a scheduled date or a deadline date that matches the given
    /// criterion. Items which would match because they have neither a scheduled date nor a
    /// deadline will not be shown, though those that have only one will be.
    ScheduledOrDeadline,
}
impl Default for PlanningMatchType {
    fn default() -> Self {
        PlanningMatchType::All
    }
}

/// An aggregation of the views provided by their data types. Each view has its name associated.
#[derive(Debug)]
pub struct AllViews {
    pub events: Vec<(String, EventsFilter)>,
    pub daily_notes: Vec<(String, DailyNotesFilter)>,
    pub tickles: Vec<(String, TicklesFilter)>,
    pub dates: Vec<(String, DatesFilter)>,
    pub waits: Vec<(String, WaitsFilter)>,
    pub projects: Vec<(String, ProjectsFilter)>,
    pub tasks: Vec<(String, TasksFilter)>,
    pub target_contexts: Vec<(String, TargetContextsFilter)>,

    /// The latest date across all the views, if there is one (the user might have specified only
    /// non-date filters). This will be used to define when to stop expanding repeating timestamps
    /// (after a buffer is added).
    pub last_date: Option<NaiveDate>,
}
