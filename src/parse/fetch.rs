use super::node::{Format, Node, NodeOptions};
use anyhow::{bail, Context, Result};

/// Gets the raw nodes from the given Starling endpoint, filtering automatically to those that meet
/// the next actions filter (i.e. those with timestamps, keywords, etc.). This will override part
/// of the provided [`NodeOptions`] to fetch metadata and children, also formatting connections in
/// Markdown (later parsing requires this).
pub fn get_raw_action_items(mut opts: NodeOptions, starling_addr: &str) -> Result<Vec<Node>> {
    opts.conn_format = Format::Markdown;
    opts.children = true;
    opts.metadata = true;

    let mut res = ureq::get(&format!(
        "http://{}/index/action_items/nodes",
        starling_addr
    ))
    .config()
    .http_status_as_error(false)
    .build()
    .query("use_bincode", "true")
    .force_send_body()
    .send_json(opts)?;
    if res.status() != 200 {
        bail!(
            "failed to fetch nodes from {starling_addr}, received status {}",
            res.status()
        );
    }

    bincode::deserialize_from(res.body_mut().as_reader())
        .with_context(|| "failed to deserialize next actions from starling")
}

/// Skips the given node if it has one of the given completion keywords.
pub fn skip_complete(node: &Node, done_keywords: &[String]) -> bool {
    node.metadata
        .as_ref()
        .unwrap()
        .keyword
        .as_ref()
        .is_none_or(|k| !done_keywords.contains(k))
}

/// Removes any inactive timestamps from the node.
pub fn prune_inactive_ts(mut node: Node) -> Node {
    let old_timestamps =
        std::mem::replace(&mut node.metadata.as_mut().unwrap().timestamps, Vec::new());
    for ts in old_timestamps {
        if ts.active {
            node.metadata.as_mut().unwrap().timestamps.push(ts);
        }
    }

    if node
        .metadata
        .as_ref()
        .unwrap()
        .scheduled
        .as_ref()
        .is_some_and(|ts| !ts.active)
    {
        node.metadata.as_mut().unwrap().scheduled = None;
    }
    if node
        .metadata
        .as_ref()
        .unwrap()
        .deadline
        .as_ref()
        .is_some_and(|ts| !ts.active)
    {
        node.metadata.as_mut().unwrap().scheduled = None;
    }
    if node
        .metadata
        .as_ref()
        .unwrap()
        .closed
        .as_ref()
        .is_some_and(|ts| !ts.active)
    {
        node.metadata.as_mut().unwrap().scheduled = None;
    }

    node
}
