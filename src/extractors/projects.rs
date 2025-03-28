use crate::{ActionItem, Priority};
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// A project, which is a collection of tasks, potentially with information about when those tasks
/// should be started/finished, along with tasks that might not be actionable yet, and things that
/// are being waited on.
///
/// It is guaranteed that every project will either have at least one actionable task, or at least
/// one waiting item with a scheduled/deadline date. All items within a project are guaranteed to
/// have deadlines before the project's deadline.
#[derive(Serialize)]
pub struct Project {
    /// The ID of the node corresponding to this project.
    pub id: Uuid,
    /// The title of the project.
    pub title: String,
    /// The body of the project, if there is one.
    pub body: Option<String>,
    /// When the user should start working on this project.
    pub scheduled: Option<NaiveDateTime>,
    /// When the user must complete this project by.
    pub deadline: Option<NaiveDateTime>,
    /// The priority of this project. This will be inherited from any parent meta-projects as well.
    pub priority: Priority,
    /// The actionable tasks on this project, listed by their IDs and titles for convenience.
    pub actionable_tasks: Vec<(Uuid, String)>,
    /// The non-actionable tasks on this project, listed by their IDs and titles for convenience.
    pub next_tasks: Vec<(Uuid, String)>,
    /// The items being waited for within this project, listed by their IDs and titles for
    /// convenience.
    pub waiting: Vec<(Uuid, String)>,
}
impl Project {
    /// Converts the given action item into a series of projects, if its repeats would go on the
    /// project list. This will validate that the project has actionable tasks or scheduled waiting
    /// items.
    ///
    /// Note that this function doesn't validate the constituent tasks within the project (e.g.
    /// that they all have deadlines before the project's overall deadline), that is handled by
    /// [`crate::Task::from_action_item`].
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
        item.base()
            .repeats
            .iter()
            .map(move |repeat| {
                if let ActionItem::Project {
                    base,
                    priority,
                    computed_priority,
                    child_items,
                } = item
                {
                    let mut proj = Self {
                        id: base.id,
                        title: base.title.last().cloned().unwrap(),
                        body: base.body.clone(),
                        scheduled: repeat.scheduled,
                        deadline: repeat.deadline,
                        priority: computed_priority.unwrap_or(*priority),
                        actionable_tasks: Vec::new(),
                        next_tasks: Vec::new(),
                        waiting: Vec::new(),
                    };

                    let mut has_scheduled_wait = false;
                    let mut has_subprojects = false;
                    for child_id in child_items {
                        match map.get(child_id) {
                            Some(ActionItem::Task {
                                base, can_start, ..
                            }) => {
                                if *can_start {
                                    proj.actionable_tasks
                                        .push((base.id, base.title.last().cloned().unwrap()));
                                } else {
                                    proj.next_tasks
                                        .push((base.id, base.title.last().cloned().unwrap()));
                                }
                            }
                            Some(ActionItem::Waiting { base, .. }) => {
                                if base
                                    .repeats
                                    .first()
                                    .is_some_and(|r| r.scheduled.is_some() || r.deadline.is_some())
                                {
                                    has_scheduled_wait = true;
                                }

                                proj.waiting
                                    .push((base.id, base.title.last().cloned().unwrap()));
                            }
                            // If we have a subproject, then that's good enough, because it will
                            // have to have something actionable (or one of its subprojects will)
                            Some(ActionItem::Project { .. }) => has_subprojects = true,
                            _ => {}
                        }
                    }

                    if proj.actionable_tasks.is_empty() && (proj.waiting.is_empty() || !has_scheduled_wait) && !has_subprojects {
                        bail!(
                            "project {} must have at least one actionable task or scheduled waiting item",
                            proj.id
                        );
                    }

                    Ok(Some(proj))
                } else {
                    Ok(None)
                }
            })
            .filter_map(|res| res.transpose())
    }
}
