use std::collections::HashMap;

use crate::{ActionItem, SimpleTimestamp};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use orgish::timestamp::DateTime;
use serde::Serialize;
use uuid::Uuid;

use super::{Event, EventType};

/// A note to be displayed as something to remember on a specific day.
///
/// These are different from tasks, they're more like little notes to oneself.
#[derive(Serialize)]
pub struct DailyNote {
    /// The ID of the node corresponding to this daily note.
    pub id: Uuid,
    /// The title of this note.
    pub title: String,
    /// The body of this note, if one is present.
    pub body: Option<String>,
    /// The date on which this daily note should be displayed.
    pub date: NaiveDate,
}
impl DailyNote {
    /// Converts the given action item into a list of daily notes, if the item's repeats would go
    /// onto the list of daily notes.
    pub fn from_action_item(item: &ActionItem) -> impl Iterator<Item = Result<Self>> + '_ {
        item.base().repeats.iter().filter_map(move |repeat| {
            if let ActionItem::Note { .. } = item {
                repeat.primary.as_ref().map(|ts| {
                    if ts.end.is_some() || ts.start.time.is_some() {
                        Err(anyhow!(
                            "daily note {} is not an all-day event",
                            item.base().id
                        ))
                    } else {
                        Ok(Self {
                            id: item.base().id,
                            title: item.base().title.last().cloned().unwrap(),
                            body: item.base().body.clone(),
                            date: ts.start.date,
                        })
                    }
                })
            } else {
                None
            }
        })
    }

    /// Converts the given iterator of daily notes into a single event. This allows daily notes to
    /// be displayed in a dedicated calendar item, which can be handy. This produces a separate
    /// event for each day on which there is at least one daily note.
    pub fn notes_to_events<'a>(
        notes: impl Iterator<Item = &'a Self>,
    ) -> impl Iterator<Item = Event> {
        let mut days = HashMap::new();

        // Add each note to each day, turning them into strings for the event body
        for note in notes {
            days.entry(note.date).or_insert_with(Vec::new).push(
                format!(
                    "# {}\n{}",
                    note.title,
                    note.body.clone().unwrap_or_default()
                )
                .trim()
                .to_string(),
            );
        }

        days.into_iter().map(|(date, note_strings)| Event {
            id: Uuid::new_v4(),
            title: "📍 Daily notes".to_string(),
            body: Some(note_strings.join("\n\n")),
            location: None,
            people: Vec::new(),
            timestamp: SimpleTimestamp {
                start: DateTime { date, time: None },
                end: None,
            },
            ty: EventType::Composite,
        })
    }
}
