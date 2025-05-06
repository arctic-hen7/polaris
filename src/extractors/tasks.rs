use crate::{ActionItem, ActionItemRepeat, Effort, Priority, SimpleTimestamp};
use anyhow::{bail, Result};
use chrono::{Duration, NaiveDateTime, NaiveTime};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A task which has not been slated for a particular time, and which can be actioned immediately.
/// From the information in each task, the user can schedule them for particular times/days, or
/// simply leave them and do them when convenient.
///
/// Tasks with their own timestamps, or tasks which are part of projects with timestamps, will not
/// appear here, as they're considered handled. Non-actionable tasks, however, will.
#[derive(Serialize)]
pub struct Task {
    /// The ID of the node corresponding to this task.
    pub id: Uuid,
    /// The title of this task.
    pub title: String,
    /// The body of this task, if one exists.
    pub body: Option<String>,
    /// Whether or not this task is actionable, and can be started immediately.
    pub can_start: bool,
    /// The date by which this task should be started. If an earlier date is present on the parent
    /// project, that will be used. This is required to be before whatever the computed deadline
    /// is, or an error will be thrown.
    pub scheduled: Option<NaiveDateTime>,
    /// The date by which this task must be completed. This may be computed by several rules
    /// against the parent project and non-actionable tasks.
    pub deadline: Option<NaiveDateTime>,
    /// The priority of the task, which will be the highest priority in the path from the node
    /// corresponding to this task to the root of its file (i.e. the highest parent priority).
    pub priority: Priority,
    /// Whether or not the parent project of this task has other, non-actionable tasks. This should
    /// be displayed to the user just to make sure they don't get caught unaware.
    pub project_has_non_actionable: bool,
    /// The effort required to complete this task.
    pub effort: Effort,
    /// The contexts required to complete this task.
    pub contexts: HashSet<String>,
    /// The people needed to complete this task, listed by their IDs in the system and their
    /// names.
    pub people: Vec<(Uuid, String)>,
}
impl Task {
    /// Converts the given action item into a series of tasks, if the repeats of that item would go
    /// on the list of tasks yet to be handled.
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
        item.base()
            .repeats
            .iter()
            .enumerate()
            .map(move |(idx, repeat)| {
                if let ActionItem::Task {
                    base,
                    priority,
                    computed_priority,
                    effort,
                    contexts,
                    people,
                    can_start,
                } = item
                {
                    let (
                        ActionItemRepeat {
                            primary: parent_ts,
                            scheduled,
                            deadline,
                        },
                        has_next_tasks,
                    ) = compute_from_parent(item, idx, map)?;

                    // Now get a primary timestamp on this task or the parent, if one exists,
                    // and take the later one to make sure we will finish this task before its
                    // deadline
                    let earliest_ts = min_dt(
                        repeat.primary.as_ref().map(|ts| final_ts_point(ts)),
                        parent_ts.as_ref().map(|ts| final_ts_point(ts)),
                    );
                    if earliest_ts.is_some()
                        && deadline.is_some()
                        && earliest_ts.unwrap() > deadline.unwrap()
                    {
                        eprintln!(
                            "task {} will not be completed before its computed deadline",
                            base.id
                        );
                    }

                    // And finally only return the task if it and its parent project have not been
                    // scheduled for a particular time
                    if repeat.primary.is_some() || parent_ts.is_some() {
                        Ok(None)
                    } else {
                        Ok(Some(Self {
                            id: base.id,
                            title: base.title.last().cloned().unwrap(),
                            body: base.body.clone(),
                            can_start: *can_start,
                            scheduled,
                            deadline,
                            priority: computed_priority.unwrap_or(*priority),
                            project_has_non_actionable: has_next_tasks,
                            effort: *effort,
                            contexts: contexts.clone(),
                            people: people.clone(),
                        }))
                    }
                } else {
                    Ok(None)
                }
            })
            .filter_map(|res| res.transpose())
    }
}

/// Computes scheduled and deadline dates from the parent of the given action item. If the action
/// item is an actionable task or waiting item, this will also cross-inherit from non-actionable
/// tasks on the same project (this won't be done for other non-actionable tasks). This will also
/// return whether or not there are related non-actionable tasks on the same project as the given
/// item.
///
/// The primary timestamp on the given repeat data will be the parent's timestamp.
///
/// # Methodology
///
/// 1. Check if the task has a parent project in the map of action items
/// 2. If we don't have our own scheduled/deadline dates, use ones from the same-index repeat of
/// the parent project
/// 3. If our deadline is later that the project's (if it exists), throw an error
/// 4. Check if any of the child tasks are non-actionable
/// 5. If they are, go through them and find the earliest scheduled/deadline date among them
/// 6. If there wasn't one, use the project deadline instead, if it exists; call *one day before*
///    whichever we used the computed deadline
/// 7. If there were *no* non-actionable tasks, set the computed deadline to the project deadline
///    itself
/// 8. If we have a computed deadline, set our deadline to the earlier of what it originally was,
///    and the computed one
/// 9. If the scheduled date we have is after whatever our deadline now is, throw an error (noting
///    whether or not the deadline was computed)
/// 10. Take the later of a timestamp on this task and a timestamp on the parent project, if either
///     exist, and accumulate a warning if that timestamp is after what's now this task's deadline
///
/// # Panics
///
/// This function will panic if the given repeat index does not exist on the given action item.
pub fn compute_from_parent(
    item: &ActionItem,
    repeat_idx: usize,
    map: &HashMap<Uuid, ActionItem>,
) -> Result<(ActionItemRepeat, bool)> {
    let repeat = &item.base().repeats[repeat_idx];

    let mut scheduled = repeat.scheduled;
    let mut deadline = repeat.deadline;

    let mut parent_ts = None;
    let mut has_next_tasks = false;
    if let Some(parent @ ActionItem::Project { child_items, .. }) =
        item.base().parent_id.map(|id| map.get(&id)).flatten()
    {
        // Just blindly rely on the same-index repeat from the parent, there's not
        // really much else we can do. If it doesn't exist, treat it like there isn't a
        // parent
        if let Some(parent_repeat) = parent.base().repeats.get(repeat_idx) {
            parent_ts = parent_repeat.primary.clone();

            // Inherit the parent project's scheduled/deadline dates if we don't have
            // our own
            if scheduled.is_none() && parent_repeat.scheduled.is_some() {
                scheduled = parent_repeat.scheduled.clone();
            }
            if deadline.is_none() && parent_repeat.deadline.is_some() {
                deadline = parent_repeat.deadline.clone();
            }

            // Make sure our deadline is before the project's
            if parent_repeat.deadline.is_some()
                && deadline.unwrap() > parent_repeat.deadline.unwrap()
            {
                bail!(
                    "item {} has a deadline after that of its parent project",
                    item.base().id
                );
            }

            // For actionable tasks and waiting items, we should compute their deadline to be
            // before we need to start any non-actionable tasks
            if matches!(
                item,
                ActionItem::Task {
                    can_start: true,
                    ..
                } | ActionItem::Waiting { .. }
            ) {
                // Check if any of the other children of this project are non-actionable
                // tasks (we don't need to change any of our behaviour for waiting items).
                // If there are some, find the earliest scheduled/deadline date among them,
                // falling back to the project deadline, if there is one.
                let mut earliest_imposed_deadline = parent_repeat.deadline.min(repeat.deadline);
                for child_id in child_items {
                    if let ActionItem::Task {
                        can_start: false, ..
                    } = map.get(child_id).unwrap()
                    {
                        has_next_tasks = true;

                        // Match the repeat, as with the parent
                        if let Some(child_repeat) =
                            map.get(child_id).unwrap().base().repeats.get(repeat_idx)
                        {
                            earliest_imposed_deadline =
                                min_dt(earliest_imposed_deadline, child_repeat.scheduled);
                            earliest_imposed_deadline =
                                min_dt(earliest_imposed_deadline, child_repeat.deadline);
                        }
                    }
                }

                // If we have an imposed deadline from there, set it. If we have
                // non-actionable tasks make it one day before whenever we have to start
                // those to provide wiggle room. We don't need to do this if we aren't checking
                // cross-inheritance, because we wouldn't have any next tasks, and the deadline
                // is already guaranteed to be before that of the parent project.
                if let Some(imposed_deadline) = earliest_imposed_deadline {
                    deadline = Some(if has_next_tasks {
                        imposed_deadline - Duration::days(1)
                    } else {
                        imposed_deadline
                    });
                }
            }
        }
    }

    // Now make sure the scheduled date we have (maybe inherited) is before the
    // computed deadline
    if scheduled.is_some() && deadline.is_some() && scheduled.unwrap() > deadline.unwrap() {
        bail!(
            "item {} has a scheduled date before its computed deadline date",
            item.base().id
        );
    }

    Ok((
        ActionItemRepeat {
            primary: parent_ts,
            scheduled,
            deadline,
        },
        has_next_tasks,
    ))
}

/// Returns the earlier of the two given datetimes, either of which may be `None`.
fn min_dt(a: Option<NaiveDateTime>, b: Option<NaiveDateTime>) -> Option<NaiveDateTime> {
    match (a, b) {
        (None, None) => None,
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (Some(a), Some(b)) => Some(a.min(b)),
    }
}

/// Returns the final datetime in the given timestamp.
fn final_ts_point(ts: &SimpleTimestamp) -> NaiveDateTime {
    let dt = if let Some(end) = &ts.end {
        end
    } else {
        &ts.start
    };
    dt.date.and_time(
        dt.time
            .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
    )
}
