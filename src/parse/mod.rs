mod action_item;
mod fetch;
mod node;
mod repeat;

use anyhow::Result;
use chrono::NaiveDate;
use fetch::{prune_inactive_ts, skip_complete};
use node::Node;
use repeat::expand_timestamps;
use std::collections::HashMap;
use uuid::Uuid;

pub use action_item::*;
pub use fetch::get_raw_action_items;
pub use node::*;

/// Normalises the given raw nodes to a list of parsed action items, repeated until the given date.
pub fn normalize_action_items(
    nodes: Vec<Node>,
    done_keywords: &[String],
    until: NaiveDate,
) -> Result<HashMap<Uuid, ActionItem>> {
    let mut map = nodes
        .into_iter()
        .filter(|n| skip_complete(n, done_keywords))
        .map(prune_inactive_ts)
        .map(|n| (n.id, expand_timestamps(&n, until).collect::<Vec<_>>(), n))
        .map(|(id, repeats, node)| node_to_action_item(node, repeats).map(|item| (id, item)))
        .collect::<Result<HashMap<Uuid, ActionItem>>>()?;
    let ids = map.keys().copied().collect::<Vec<_>>();
    for id in ids {
        fill_action_item(id, &mut map);
    }

    Ok(map)
}
