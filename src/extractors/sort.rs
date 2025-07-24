use super::{DailyNote, Event, PersonDate, Project, Task, Tickle, Waiting};
use crate::parse::Priority;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// The end of representable time, used as a maximum time to push items without scheduled or
/// deadline dates to the end of a sorted list.
const END_OF_TIME: NaiveDateTime = NaiveDateTime::MAX;

/// A sorting structure for types with scheduled and deadline dates. The sorting behaviour here is
/// somewhat unique, so we abstract it.
pub struct ScheduledDeadline {
    first: NaiveDateTime,
    second: NaiveDateTime,
}
impl ScheduledDeadline {
    /// Creates a new sorting key for the given scheduled/deadline datetimes.
    pub fn new(scheduled: Option<NaiveDateTime>, deadline: Option<NaiveDateTime>) -> Self {
        Self {
            // The effect here is to sort by the "first-priority" date always. If an item has a
            // scheduled date, that will be it, otherwise its deadline will be. If it has neither,
            // both `first` and `second` will be `END_OF_TIME`, and will be placed at the end.
            first: scheduled.unwrap_or(deadline.unwrap_or(END_OF_TIME)),
            second: deadline.unwrap_or(END_OF_TIME),
        }
    }
}
impl Ord for ScheduledDeadline {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.first
            .cmp(&other.first)
            .then_with(|| self.second.cmp(&other.second))
    }
}
impl PartialOrd for ScheduledDeadline {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for ScheduledDeadline {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.second == other.second
    }
}
impl Eq for ScheduledDeadline {}

// TODO: Is there a way to avoid all these string clones?
impl Event {
    pub fn sort_key(&self) -> (NaiveDate, Option<NaiveTime>, String) {
        (
            self.timestamp.start.date,
            self.timestamp.start.time,
            self.title.clone(),
        )
    }
}

impl DailyNote {
    pub fn sort_key(&self) -> (NaiveDate, String) {
        (self.date, self.title.clone())
    }
}

impl Tickle {
    pub fn sort_key(&self) -> (NaiveDate, String) {
        (self.date, self.title.clone())
    }
}

impl PersonDate {
    pub fn sort_key(&self) -> (NaiveDate, NaiveDate, String) {
        (self.notify_date, self.date, self.title.clone())
    }
}

impl Waiting {
    pub fn sort_key(&self) -> (ScheduledDeadline, String) {
        (
            ScheduledDeadline::new(self.scheduled, self.deadline),
            self.title.clone(),
        )
    }
}

impl Project {
    pub fn sort_key(&self) -> (NaiveDate, NaiveTime, ScheduledDeadline, Priority, String) {
        (
            self.timestamp
                .as_ref()
                .map(|ts| ts.start.date)
                .unwrap_or(END_OF_TIME.date()),
            self.timestamp
                .as_ref()
                .and_then(|ts| ts.start.time)
                .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
            ScheduledDeadline::new(self.scheduled, self.deadline),
            self.priority,
            self.title.clone(),
        )
    }
}

impl Task {
    pub fn sort_key(
        &self,
    ) -> (
        NaiveDate,
        NaiveTime,
        NaiveDate,
        NaiveTime,
        ScheduledDeadline,
        Priority,
        String,
    ) {
        (
            self.timestamp
                .as_ref()
                .map(|ts| ts.start.date)
                .unwrap_or(END_OF_TIME.date()),
            self.timestamp
                .as_ref()
                .and_then(|ts| ts.start.time)
                .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
            self.parent_timestamp
                .as_ref()
                .map(|ts| ts.start.date)
                .unwrap_or(END_OF_TIME.date()),
            self.parent_timestamp
                .as_ref()
                .and_then(|ts| ts.start.time)
                .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
            ScheduledDeadline::new(self.scheduled, self.deadline),
            self.priority,
            self.title.clone(),
        )
    }
}
