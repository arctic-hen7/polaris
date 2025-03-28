use super::tasks::compute_from_parent;
use crate::{ActionItem, ActionItemRepeat};
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Something being waited for. These will usually either exist in isolation, or as part of
/// projects, before `NEXT` tasks. As such, like actionable tasks, the scheduled and deadline dates
/// of waiting items will be adjusted for their parent project's non-actionable tasks.
#[derive(Serialize)]
pub struct Waiting {
    /// The ID of the node corresponding to this waiting item.
    pub id: Uuid,
    /// The title of the waiting item.
    pub title: String,
    /// The body of the waiting item, if there is one.
    pub body: Option<String>,
    /// The date on which the obligation to complete this was delegated to someone else.
    pub sent: NaiveDate,
    /// The date on which the user should start thinking about chasing up a response.
    pub scheduled: Option<NaiveDateTime>,
    /// The date by which the user needs to have a response.
    pub deadline: Option<NaiveDateTime>,
}
impl Waiting {
    /// Converts the given action item into a series of waiting items, if the item's repeats would
    /// go onto the list of waiting items.
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
        item.base()
            .repeats
            .iter()
            .enumerate()
            .map(move |(idx, _)| {
                if let ActionItem::Waiting { base, sent } = item {
                    // Compute the scheduled/deadline dates as we do for tasks. We don't need to
                    // check the timestamps though, because waiting items can't be put into the
                    // events list.
                    let (
                        ActionItemRepeat {
                            primary: _,
                            scheduled,
                            deadline,
                        },
                        _,
                    ) = compute_from_parent(item, idx, map)?;

                    Ok(Some(Self {
                        id: base.id,
                        title: base.title.last().cloned().unwrap(),
                        body: base.body.clone(),
                        sent: *sent,
                        scheduled,
                        deadline,
                    }))
                } else {
                    Ok(None)
                }
            })
            .filter_map(|res| res.transpose())
    }
}
