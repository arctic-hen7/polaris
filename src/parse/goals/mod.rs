//! This module extracts goals, whose format is *highly* unique to my system. If you use a
//! different system, you will either need to modify this file's extraction systems, or just ignore
//! this part of Polaris entirely. Deliberately, goal checks will only be run if you explicitly
//! request them (unlike the rest of the system, which validates everything no matter what you
//! request).

mod personal;

use super::NodeOptions;
use crate::parse::Node;
use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::VecDeque;
use uuid::Uuid;

/// A list of goals for a single day.
#[derive(Serialize, Debug)]
pub struct Goals {
    /// The date for which these goals were extracted.
    date: NaiveDate,
    /// The types of goals. As different people have different systems for goals, this can extract
    /// arbitrarily many different "types" of goals that come from different places. Keys here are
    /// human-readable things like *Daily Goals* or *Weekly Goals*, and the values are lists of the
    /// actual goals that have been set. This is stored as a vector rather than a map to allow
    /// custom ordering.
    goals: Vec<(String, Vec<String>)>,
}
impl Goals {
    /// Extracts goals for the given date.
    pub fn extract(date: NaiveDate, starling_addr: &str) -> Result<Self> {
        // Get the goal types/sources for this date, then convert them into real goals
        let goals = personal::goals_for_date(date)
            .into_iter()
            .map(|(name, goals_source)| {
                goals_source
                    .into_goals(starling_addr)
                    .map(|goals| (name, goals))
            })
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("failed to extract goals for date {date} from sources"))?;

        Ok(Self { date, goals })
    }
}
impl Default for Goals {
    fn default() -> Self {
        // Default to an empty goals list for today
        Self {
            date: NaiveDate::MIN,
            goals: Vec::new(),
        }
    }
}

/// Different places goals can come from. This provides an abstraction over the final approach of
/// extracting goals from a node with a known ID and allows for more "natural" strategies like
/// specifying a daily journal file and a heading inside it where goals can be found.
///
/// Wherever goals are finally found, they will be extracted as a Markdown list (starting with
/// `- `) from the body of the final node that's found.
///
/// Additional strategies will likely be added here in future.
#[non_exhaustive]
pub(super) enum GoalsSource {
    /// Goals will be found in the body of the node with this ID.
    Id(Uuid),
    /// Goals will be found in the file with this path, inside the heading with this name.
    File {
        /// The path to the file where goals can be found. This must be specified such that
        /// Starling can understand it, so it should be *relative to the Starling root*. If this
        /// isn't true, goal extraction will almost certainly fail.
        path: String,
        /// The path to the heading *inside* that file. For example, if you're looking for the
        /// top-level heading *Daily Goals*, you would just provide
        /// `vec!["Daily Goals".to_string()]`. But if you were looking for the second-level heading
        /// *Daily* inside the top-level *Goals* heading, you would provide
        /// `vec!["Goals".to_string(), "Daily".to_string()]`.
        ///
        /// This is essentially the full Starling heading path to your node, but minus the title of
        /// the file itself. If you want to read from the root body in your file, you can provide
        /// an empty vector here.
        heading_path: Vec<String>,
        /// If true, goal extraction will outright fail if the heading specified isn't present in
        /// the file. If false, it will just return an empty list of goals.
        ///
        /// Generally, you should set this to `true` unless you know this heading might be missing
        /// and that's okay.
        fail_on_missing_heading: bool,
    },
}
impl GoalsSource {
    /// Converts this [`GoalsSource`] into the actual goals it references.
    fn into_goals(self, starling_addr: &str) -> Result<Vec<String>> {
        // Helper function to get the details of the node with the given ID
        fn get_node_details(
            node_id: Uuid,
            diagnostic_title: &str,
            starling_addr: &str,
        ) -> Result<Node> {
            // We'll get both the children in case we need to do further traversal, and the
            // body in case this is the last node in the path
            let mut opts = NodeOptions::default();
            opts.body = true;
            opts.children = true;

            let mut res = ureq::get(&format!("http://{starling_addr}/node/{node_id}"))
                .config()
                .http_status_as_error(false)
                .build()
                .query("use_bincode", "false")
                .force_send_body()
                .send_json(opts)?;
            if res.status() != 200 {
                bail!(
                    "failed to get node details for node {node_id} (\"{diagnostic_title}\"), received status {}",
                    res.status()
                );
            }

            let node_details: Node = serde_json::from_reader(res.body_mut().as_reader())
                .with_context(|| format!("failed to deserialize node details from starling for node {node_id} (\"{diagnostic_title}\")"))?;
            Ok(node_details)
        }

        let body = match self {
            GoalsSource::Id(id) => get_node_details(id, "RAW ID GIVEN", starling_addr)
                .map(|node| node.body.unwrap())?,
            GoalsSource::File {
                path,
                heading_path,
                fail_on_missing_heading,
            } => {
                let mut heading_path = VecDeque::from(heading_path);

                // Quick sanity check that the path is relative to Starling
                if path.starts_with("/") || path.starts_with("~") {
                    bail!("goal file path must be relative to the starling root, but got: {path} (also should not start with `/`)");
                }

                // Get the root ID of that path (no `bincode` support on this endpoint)
                let path_url = urlencoding::encode(&path);
                let mut res = ureq::get(&format!("http://{starling_addr}/root-id/{path_url}"))
                    .config()
                    .http_status_as_error(false)
                    .build()
                    .call()?;
                if res.status() != 200 {
                    bail!(
                        "failed to get root id for file {path}, received status {}",
                        res.status()
                    );
                }
                let root_id: String = serde_json::from_reader(res.body_mut().as_reader())
                    .with_context(|| {
                        format!("failed to deserialize root id from starling for file {path}")
                    })?;
                let root_id = Uuid::parse_str(&root_id).with_context(|| {
                    format!("failed to parse root id {root_id} for file {path}")
                })?;

                // Now get the details of the root ID, and go through the heading path until we
                // find the right node
                let mut current_node =
                    get_node_details(root_id, &format!("root of {path}"), starling_addr)?;
                while let Some(next_title) = heading_path.pop_front() {
                    let mut next_id = None;
                    for (child_id, child_title) in current_node.children {
                        if child_title == next_title {
                            next_id = Some(child_id);
                            break;
                        }
                    }
                    if let Some(next_id) = next_id {
                        current_node = get_node_details(
                            next_id,
                            &format!("heading {next_title} in {path}"),
                            starling_addr,
                        )?;
                    } else if fail_on_missing_heading {
                        bail!(
                            "failed to find heading {next_title} in file {path}, which is required for goal extraction (`fail_on_missing_heading` was set to `true`)"
                        );
                    } else {
                        // If we're not failing on missing headings, we can just return an empty
                        // body here
                        return Ok(vec![]);
                    }
                }

                current_node.body.unwrap()
            }
        };

        // Parse the body
        Ok(body
            .lines()
            .map(|l| l.trim())
            // We only want lines starting with `- ` (implicitly filters out trimmed empty lists with
            // just `-` as well as empty ones)
            .filter_map(|l| l.strip_prefix("- "))
            .map(|l| l.to_string())
            .collect::<Vec<_>>())
    }
}
