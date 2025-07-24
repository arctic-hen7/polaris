use crate::ActionItem;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// A note to be displayed as something to remember on a specific day.
///
/// These are different from tasks, they're more like little notes to oneself.
#[derive(Serialize, Clone, Debug)]
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
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        _map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
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
}
