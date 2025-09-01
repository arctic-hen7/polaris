use crate::{parse::SimpleTimestamp, ActionItem, Priority};
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::{Task, Waiting};

/// A stack, which is a collection of tasks, potentially with information about when those tasks
/// should be started/finished, along with tasks that might not be actionable yet, and things that
/// are being waited on.
///
/// It is guaranteed that every stack will either have at least one actionable task, or at least
/// one waiting item with a scheduled/deadline date. All items within a stack are guaranteed to
/// have deadlines before the stack's deadline.
///
/// Conceptually, stacks are like stacks. They're designed particularly for tasks that don't
/// themselves have any information about when they need to be done, and the best way I find of
/// handling them is to just put them in a kind of "holding tank"/"conveyor belt" that I can pull
/// from when I want to, or need to, work in that particular area.
#[derive(Serialize, Clone, Debug)]
pub struct Stack {
    /// The ID of the node corresponding to this stack.
    pub id: Uuid,
    /// The title of the stack.
    pub title: String,
    /// The body of the stack, if there is one.
    pub body: Option<String>,
    /// The main timestamp of the stack, indicating when to next work on it, if it has one.
    pub timestamp: Option<SimpleTimestamp>,
    /// When the user should start working on this stack.
    pub scheduled: Option<NaiveDateTime>,
    /// When the user must complete this stack by.
    pub deadline: Option<NaiveDateTime>,
    /// The priority of this stack. This will be inherited from any parent meta-stacks as well.
    pub priority: Priority,
    /// The actionable tasks on this stack, fully parsed for convenience.
    pub actionable_tasks: Vec<Task>,
    /// The non-actionable tasks on this stack, fully parsed for convenience.
    pub next_tasks: Vec<Task>,
    /// The items being waited for within this stack, fully parsed for convenience.
    pub waiting: Vec<Waiting>,
}
impl Stack {
    /// Converts the given action item into a series of stacks, if its repeats would go on the
    /// stack list. This will validate that the stack has actionable tasks or scheduled waiting
    /// items.
    ///
    /// Note that this function doesn't validate the constituent tasks within the stack (e.g.
    /// that they all have deadlines before the stack's overall deadline), that is handled by
    /// [`crate::Task::from_action_item`].
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
        item.base()
            .repeats
            .iter()
            .map(move |repeat| {
                if let ActionItem::Stack {
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
                        timestamp: repeat.primary.clone(),
                        scheduled: repeat.scheduled,
                        deadline: repeat.deadline,
                        priority: computed_priority.unwrap_or(*priority),
                        actionable_tasks: Vec::new(),
                        next_tasks: Vec::new(),
                        waiting: Vec::new(),
                    };

                    // We keep track of scheduled waiting items of substacks to check if this stack
                    // has something to *do* on it (if not, it's inherently invalid)
                    let mut has_scheduled_wait = false;
                    let mut has_substacks = false;
                    for child_id in child_items {
                        match map.get(child_id) {
                            Some(item @ ActionItem::Task { .. }) => {
                                // Process all subtasks and get their first repeat (guaranteed to
                                // exist)
                                let task = Task::from_action_item(item, map).next().unwrap()?;

                                if task.can_start {
                                    proj.actionable_tasks
                                        .push(task);
                                } else {
                                    proj.next_tasks
                                        .push(task);
                                }
                            }
                            Some(item @ ActionItem::Waiting { .. }) => {
                                // Similarly, process waiting-for items
                                let waiting = Waiting::from_action_item(item, map)
                                    .next()
                                    .unwrap()?;

                                // We'll note down if there's a waiting item with a scheduled or
                                // deadline date
                                if waiting.scheduled.is_some() || waiting.deadline.is_some() {
                                    has_scheduled_wait = true;
                                }

                                proj.waiting
                                    .push(waiting);
                            }
                            // If we have a substack, then that's good enough, because it will
                            // have to have something actionable (or one of its substacks will)
                            Some(ActionItem::Stack { .. }) => has_substacks = true,
                            _ => {}
                        }
                    }

                    if proj.actionable_tasks.is_empty() && (proj.waiting.is_empty() || !has_scheduled_wait) && !has_substacks {
                        bail!(
                            "stack {} must have at least one actionable task or scheduled waiting item",
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
