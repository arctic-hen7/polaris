use crate::parse::{ActionItem, SimpleTimestamp};
use serde::Serialize;
use std::{collections::HashMap, convert::Infallible};
use uuid::Uuid;

/// An event, to be held/attended at a specific time.
#[derive(Serialize, Clone, Debug)]
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
}
impl Event {
    /// Converts the given action item into events, if its repeats would go on the calendar.
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        _map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self, Infallible>> + 'a {
        item.base().repeats.iter().filter_map(move |repeat| {
            // No person-related dates, tickles, daily notes, or waiting items are events. This
            // check is the same every time, so should get hoisted out of the loop
            let parent_tags = &item.base().parent_tags;
            if parent_tags.contains("person_dates") || parent_tags.contains("tickles") {
                // No person-related dates or tickles are events
                None
            } else if let ActionItem::None {
                base,
                properties,
                people,
            } = item
            {
                repeat.primary.as_ref().map(|ts| {
                    Ok(Self {
                        id: base.id,
                        title: base.title.last().cloned().unwrap(),
                        body: base.body.clone(),
                        location: properties.get("LOCATION").cloned(),
                        people: people.clone(),
                        timestamp: ts.clone(),
                    })
                })
            } else {
                // No daily notes, waiting items, tasks, or projects are events
                // NOTE: Used to be that we would catch tasks and projects with timestamps, they're
                // now handled in their own pipelines.
                None
            }
        })
    }
}
