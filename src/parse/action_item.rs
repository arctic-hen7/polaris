use super::node::Node;
use anyhow::{anyhow, bail, Result};
use chrono::{NaiveDate, NaiveDateTime};
use clap::ValueEnum;
use orgish::timestamp::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Converts the given node into its corresponding action item. This does not complete the process,
/// and a second passthrough against a map of all the action items will be needed to fill in
/// connecting details and computed relative properties.
pub fn node_to_action_item(node: Node, repeats: Vec<ActionItemRepeat>) -> Result<ActionItem> {
    let base = BaseActionItem {
        id: node.id,
        title: node.title.clone(),
        body: node.body.clone(),
        parent_tags: node.parent_tags.clone(),
        parent_id: node.parent_id,
        repeats,
    };

    match &node.metadata.as_ref().unwrap().keyword {
        Some(kw) => {
            match kw.as_str() {
                "TODO" | "NEXT" => Ok(ActionItem::Task {
                    base,

                    people: people_from_node(&node)?,
                    priority: Priority::from_node(&node)?,
                    computed_priority: None, // Later
                    effort: Effort::from_node(&node)?,
                    contexts: node.tags.clone(),
                    can_start: kw == "TODO",
                }),
                "WAIT" => Ok(ActionItem::Waiting {
                    base,
                    sent: node
                        .metadata
                        .as_ref()
                        .unwrap()
                        .properties
                        .get("SENT")
                        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
                        .ok_or(anyhow!("no SENT property on waiting node {}", node.id))??,
                }),
                "NOTE" => Ok(ActionItem::Note { base }),
                "PROJ" => {
                    // Make sure there is at least one actionable item in this project (i.e. one
                    // `TODO`)

                    Ok(ActionItem::Project {
                        base,
                        priority: Priority::from_node(&node)?,
                        computed_priority: None, // Later
                        child_items: node.children.iter().map(|(id, _)| *id).collect(), // Later
                    })
                }
                _ => bail!("unknown keyword: {kw}"),
            }
        }
        None => Ok(ActionItem::None {
            base,
            people: people_from_node(&node)?,
            properties: node.metadata.as_ref().unwrap().properties.clone(),
        }),
    }
}

/// Completes the details of the action item with the given ID in the given map. This will inherit
/// priorities and compute artificial timestamps needed for scheduling, as well as fill in the
/// details of related nodes.
///
/// # Panics
///
/// This function will panic if the given ID is not in the map.
pub fn fill_action_item(id: Uuid, map: &mut HashMap<Uuid, ActionItem>) {
    let mut item = map.remove(&id).unwrap();

    match &mut item {
        ActionItem::Task {
            base,
            priority,
            computed_priority,
            ..
        } => {
            // If there's a parent node, try to compute its priority recursively, and if that's
            // higher than our own, set our computed priority
            if let Some(parent_id) = base.parent_id {
                let parent_computed_priority = inherit_priority(parent_id, map);
                if parent_computed_priority.is_some_and(|p| p > *priority) {
                    *computed_priority = parent_computed_priority;
                }
            }
        }
        ActionItem::Project {
            base,
            priority,
            computed_priority,
            child_items,
        } => {
            // If there's a parent node, try to compute its priority recursively, and if that's
            // higher than our own, set our computed priority
            if let Some(parent_id) = base.parent_id {
                let parent_computed_priority = inherit_priority(parent_id, map);
                if parent_computed_priority.is_some_and(|p| p > *priority) {
                    *computed_priority = parent_computed_priority;
                }
            }

            // Filter the children down to only ones that are in the map
            *child_items = child_items
                .drain(..)
                .filter(|id| map.get(&id).is_some())
                .collect();
        }

        _ => {}
    };

    map.insert(id, item);
}

/// An action item within the task management system.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionItem {
    Project {
        base: BaseActionItem,

        /// The priority of this project. If a higher value is present in the task's parent
        /// project(s), that value will be used.
        priority: Priority,
        /// A higher priority computed for this project by looking at the priorities of parent
        /// projects.
        ///
        /// This is computed in the second passthrough, and will initially be `false`.
        computed_priority: Option<Priority>,
        /// The IDs of the child action items under this project (including `WAIT` items).
        ///
        /// In the first pass, the IDs of all the children will be listed, and this will be
        /// filtered and resolved to real tasks in the second pass.
        child_items: Vec<Uuid>,
    },
    Task {
        base: BaseActionItem,

        /// The priority of this task. If a higher value is present in the task's parent
        /// project(s), that value will be used.
        priority: Priority,
        /// A higher priority computed for this task by looking at the priorities of parent
        /// projects.
        ///
        /// This is computed in the second passthrough, and will initially be `false`.
        computed_priority: Option<Priority>,
        /// The effort required to complete this task.
        effort: Effort,
        /// The contexts required to complete this task.
        contexts: HashSet<String>,
        /// The people needed to complete this task, listed by their IDs in the system and their
        /// names.
        people: Vec<(Uuid, String)>,
        /// Whether or not this task can be immediately started yet or not. Those which can be have
        /// the keyword `TODO`, and those which don't have the keyword `NEXT`.
        can_start: bool,
    },
    Waiting {
        base: BaseActionItem,

        /// The date on which the item was sent (and entered a waiting state).
        sent: NaiveDate,
    },
    Note {
        base: BaseActionItem,
        // We don't store the date because it might have repeats
    },
    None {
        base: BaseActionItem,

        /// Any properties this item has.
        properties: HashMap<String, String>,
        /// The people associated with this item, listed by their IDs in the system and their
        /// names.
        people: Vec<(Uuid, String)>,
    },
}
impl ActionItem {
    /// Gets the base properties of this action item.
    pub fn base(&self) -> &BaseActionItem {
        match &self {
            Self::Task { base, .. }
            | Self::Project { base, .. }
            | Self::Waiting { base, .. }
            | Self::Note { base, .. }
            | Self::None { base, .. } => base,
        }
    }
}

/// The base properties all action items have.
#[derive(Serialize)]
pub struct BaseActionItem {
    /// The unique ID of the item.
    pub id: Uuid,
    /// The title of the item (last element), and the titles of all its parents.
    pub title: Vec<String>,
    /// The body of the item, if present.
    pub body: Option<String>,
    /// Any tags on the parent nodes of this action item.
    pub parent_tags: HashSet<String>,
    /// The ID of the parent node, if there is one.
    pub parent_id: Option<Uuid>,
    /// The repeats of this action item.
    pub repeats: Vec<ActionItemRepeat>,
}

/// Information about a single repeat of an action item. The only things that guide a repeat are
/// the scheduled, deadline, and primary timestamps, so we only need to store those for each
/// repeat.
#[derive(Serialize, Debug)]
pub struct ActionItemRepeat {
    /// The primary timestamp (from the heading).
    pub primary: Option<SimpleTimestamp>,
    /// A datetime at which to start displaying the item to the user, if one is present.
    pub scheduled: Option<NaiveDateTime>,
    /// A datetime at which the item must be completed, if one is present.
    pub deadline: Option<NaiveDateTime>,
}

/// A simple timestamp, which is always active, and which has no repeater.
#[derive(Serialize, Clone, Debug)]
pub struct SimpleTimestamp {
    /// The date and optional time when the timestamp begins.
    pub start: DateTime,
    /// The optional date and double-optional time when the timestamp ends.
    pub end: Option<DateTime>,
}

/// The effort a task is estimated to take.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Total = 4,
    High = 3,
    Medium = 2,
    Low = 1,
    Minimal = 0,
}
impl Effort {
    /// Parses an effort level from the given node.
    fn from_node(node: &Node) -> Result<Self> {
        match node
            .metadata
            .as_ref()
            .unwrap()
            .properties
            .get("EFFORT")
            .map(|s| s.as_str())
        {
            Some("total") => Ok(Self::Total),
            Some("high") => Ok(Self::High),
            Some("medium") => Ok(Self::Medium),
            Some("med") => Ok(Self::Medium),
            Some("low") => Ok(Self::Low),
            Some("minimal") => Ok(Self::Minimal),
            Some("min") => Ok(Self::Minimal),
            Some(e) => bail!("unknown effort '{e}' on node {}", node.id),
            None => Ok(Self::Medium),
            // None => bail!("no effort level specified for node {}", node.id),
        }
    }

    // NOTE: This was used for the CLI effort filters, might add them back in future, so keeping
    // for now.
    //
    // /// Converts the given string into an effort, if possible.
    // pub fn from_str(s: &str) -> Result<Self> {
    //     match s.to_lowercase().as_str() {
    //         "total" => Ok(Self::Total),
    //         "high" => Ok(Self::High),
    //         "medium" => Ok(Self::Medium),
    //         "med" => Ok(Self::Medium),
    //         "low" => Ok(Self::Low),
    //         "minimal" => Ok(Self::Minimal),
    //         "min" => Ok(Self::Minimal),
    //         e => bail!("unknown effort '{e}'"),
    //     }
    // }
}

/// The priority of a task or project.
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, ValueEnum, Debug)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "snake_case")]
pub enum Priority {
    // NOTE: These values are *not* the same as what you'll see in priority tags!! They're used
    // only to tell the compiler how to order the variants.
    Important = 3,
    High = 2,
    Medium = 1,
    Low = 0,
}
impl Priority {
    /// Parses a priority from the given node.
    fn from_node(node: &Node) -> Result<Self> {
        match node.metadata.as_ref().unwrap().priority.as_deref() {
            Some("1") => Ok(Priority::Important),
            Some("2") => Ok(Priority::High),
            Some("3") => Ok(Priority::Medium),
            Some("4") => Ok(Priority::Low),
            Some(p) => bail!("unknown priority '{p}' on node {}", node.id),
            None => Ok(Priority::Medium),
        }
    }
}

/// Parses a list of people, by their IDs and names, from the given node.
///
/// People should be given in a `PEOPLE` property of the form `[Person 1](their-id), [Person
/// 2](their-id)`.
fn people_from_node(node: &Node) -> Result<Vec<(Uuid, String)>> {
    match node.metadata.as_ref().unwrap().properties.get("PEOPLE") {
        Some(people) => people
            .split(", ")
            .map(|p| {
                let mut parts = p.splitn(2, "](");
                let name = parts
                    .next()
                    .unwrap() // Guaranteed in a split
                    .strip_prefix("[")
                    .ok_or(anyhow!("invalid people link format in node {}", node.id))?
                    .to_string();
                let id = Uuid::parse_str(
                    parts
                        .next()
                        .ok_or(anyhow!("invalid people link format in node {}", node.id))?
                        .strip_suffix(")")
                        .ok_or(anyhow!("invalid people link format in node {}", node.id))?,
                )?;

                // A convention in my personal systems for people nodes
                let name = name.strip_prefix("(Person) ").unwrap_or(&name).to_string();

                Ok::<_, anyhow::Error>((id, name))
            })
            .collect(),
        None => Ok(Vec::new()),
    }
}

/// Computes the priority of the action item with the given ID by looking recursively through its
/// parent projects to find the highest priority. Even though recursive schedule-involved projects
/// are not used in the system, this is done to allow "meta-projects" to be given priorities that
/// filter through to their underlying tasks.
///
/// This will return `None` if the given ID is not in the map.
fn inherit_priority(id: Uuid, map: &HashMap<Uuid, ActionItem>) -> Option<Priority> {
    let mut highest_priority = Priority::Low;
    let mut current = Some(map.get(&id)?);
    while let Some(ActionItem::Project { priority, .. }) = current {
        if *priority > highest_priority {
            highest_priority = *priority;
        }
        current = current
            .as_ref()
            .unwrap()
            .base()
            .parent_id
            // If we have a parent ID, get the parent node, and it's guaranteed to have a first repeat
            // (like all elements in the map)
            .map(|parent_id| map.get(&parent_id))
            .flatten();
    }

    Some(highest_priority)
}
