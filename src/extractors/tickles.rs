use crate::ActionItem;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

/// A note for something which should re-appear in a certain day's review. This is good for things
/// that come into the inbox which should be postponed until a certain day.
///
/// Note that these should not be used for things to be remembered on a certain day (daily notes)
/// or for things being waited on (waiting-for items).
#[derive(Serialize)]
pub struct Tickle {
    /// The ID of the node associated with this tickle.
    pub id: Uuid,
    /// The title of the tickle.
    pub title: String,
    /// The body of the tickle, if there is one.
    pub body: Option<String>,
    /// The date on which this tickle should be displayed.
    pub date: NaiveDate,
}
impl Tickle {
    /// Converts the given action item into a tickle, if its repeats would go in the tickles list.
    pub fn from_action_item(item: &ActionItem) -> impl Iterator<Item = Result<Self>> + '_ {
        item.base().repeats.iter().filter_map(move |repeat| {
            if item.base().parent_tags.contains("tickles") {
                if let ActionItem::None { .. } = item {
                    repeat.primary.as_ref().map(|ts| {
                        if ts.end.is_some() || ts.start.time.is_some() {
                            Err(anyhow!("tickle {} is not an all-day event", item.base().id))
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
            } else {
                None
            }
        })
    }
}
