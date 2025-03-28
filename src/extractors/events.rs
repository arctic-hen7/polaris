use crate::parse::{ActionItem, SimpleTimestamp};
use serde::Serialize;
use uuid::Uuid;

/// An event, to be held/attended at a specific time.
#[derive(Serialize)]
pub struct Event {
    /// The unique ID of the corresponding node.
    pub id: Uuid,
    /// The title of the event.
    ///
    /// We don't need any of the parent titles, because events exist as standalone nodes.
    pub title: String,
    /// The body of the event, if there is one.
    pub body: Option<String>,
    /// The location, if there is one.
    pub location: Option<String>,
    /// Any people associated with the event.
    pub people: Vec<(Uuid, String)>,
    /// The timestamp at which the event will be occurring.
    ///
    /// TODO: Validate how range timestamps are brought over multiple days here
    pub timestamp: SimpleTimestamp,
    /// The type of the event.
    pub ty: EventType,
}
impl Event {
    /// Converts the given action item into events, if its repeats would go on the calendar.
    pub fn from_action_item(item: &ActionItem) -> impl Iterator<Item = Self> + '_ {
        item.base().repeats.iter().filter_map(move |repeat| {
            // No person-related dates, tickles, daily notes, or waiting items are events. This
            // check is the same every time, so should get hoisted out of the loop
            let parent_tags = &item.base().parent_tags;
            if parent_tags.contains("person_dates")
                || parent_tags.contains("tickles")
                || parent_tags.contains("daily_notes")
                || matches!(item, ActionItem::Waiting { .. })
            {
                None
            } else {
                repeat.primary.as_ref().map(|ts| Self {
                    id: item.base().id,
                    title: item.base().title.last().cloned().unwrap(),
                    body: item.base().body.clone(),
                    location: if let ActionItem::None { properties, .. } = item {
                        properties.get("LOCATION").cloned()
                    } else {
                        None
                    },
                    people: match item {
                        ActionItem::Task { people, .. } | ActionItem::None { people, .. } => {
                            people.clone()
                        }
                        _ => Vec::new(),
                    },
                    timestamp: ts.clone(),
                    ty: match item {
                        ActionItem::Task { .. } => EventType::Task,
                        ActionItem::Project { .. } => EventType::Project,
                        ActionItem::None { .. } => EventType::Event,

                        ActionItem::Waiting { .. } => unreachable!(),
                    },
                })
            }
        })
    }
}

/// The type of an event.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A task that's been scheduled for a specific time.
    Task,
    /// A project that's been scheduled for a specific time.
    Project,
    /// An artificial item placed on the calendar for convenience (e.g. daily notes.)
    Composite,
    /// An event proper.
    Event,
}
