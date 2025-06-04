mod cli;
mod extractors;
mod parse;

use crate::extractors::*;
use crate::parse::*;
use anyhow::Result;
use chrono::Duration;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use clap::Parser;
use cli::Cli;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;

fn main() -> Result<()> {
    let args = Cli::parse();

    let (from, until) = if args.date_range.len() == 1 {
        (None, args.date_range[0])
    } else {
        (Some(args.date_range[0]), args.date_range[1])
    };
    let min_effort = args
        .min_effort
        .map(|s| Effort::from_str(&s))
        .transpose()?
        .unwrap_or(Effort::Minimal);
    let max_effort = args
        .max_effort
        .map(|s| Effort::from_str(&s))
        .transpose()?
        .unwrap_or(Effort::Total);
    let min_priority = args
        .min_priority
        .map(|s| Priority::from_str(&s))
        .transpose()?
        .unwrap_or(Priority::Low);
    let max_priority = args
        .max_priority
        .map(|s| Priority::from_str(&s))
        .transpose()?
        .unwrap_or(Priority::Important);

    // The last date at which we'll look at everything is a bit after the last day we'll display
    // things for so we catch non-actionable tasks and are able to adjust the deadlines of
    // actionable ones to compensate. A week is usually a good distance to account for long advance
    // notification times on people-related dates as well.
    let post_until = until.and_hms_opt(23, 59, 59).unwrap() + Duration::weeks(4);

    let raw_nodes = get_raw_action_items(
        NodeOptions {
            body: true,
            metadata: true,
            children: true,
            connections: false,
            child_connections: false,
            conn_format: Format::Markdown,
        },
        &args.starling,
    )?;
    let action_items = normalize_action_items(raw_nodes, &args.done_keywords, post_until.date())?;

    // Extract every type of item, regardless of what the caller wants, because this validates
    // everything, and filter to only those within the target range
    let mut events = action_items
        .values()
        .flat_map(Event::from_action_item)
        .filter(|ev| ev.timestamp.start.date <= until)
        .filter(|ev| {
            from.is_none_or(|from| {
                ev.timestamp
                    .end
                    .as_ref()
                    .unwrap_or(&ev.timestamp.start)
                    .date
                    >= from
            })
        })
        .collect::<Vec<_>>();
    let daily_notes = action_items
        .values()
        .flat_map(DailyNote::from_action_item)
        .filter(|dn| dn.is_err() || dn.as_ref().is_ok_and(|dn| dn.date <= until))
        .filter(|dn| {
            dn.is_err()
                || dn
                    .as_ref()
                    .is_ok_and(|dn| from.is_none_or(|from| dn.date >= from))
        })
        .collect::<Result<Vec<_>>>()?;
    let tickles = action_items
        .values()
        .flat_map(Tickle::from_action_item)
        // Only filter tickles by the until date (old ones that haven't been completed should still
        // show)
        .filter(|t| t.is_err() || t.as_ref().is_ok_and(|t| t.date <= until))
        .collect::<Result<Vec<_>>>()?;
    let person_dates = action_items
        .values()
        .flat_map(PersonDate::from_action_item)
        .filter(|d| d.is_err() || d.as_ref().is_ok_and(|d| d.notify_date <= until))
        .collect::<Result<Vec<_>>>()?;
    let tasks = action_items
        .values()
        .flat_map(|item| Task::from_action_item(item, &action_items))
        // If we filter with both a scheduled and a deadline date, tasks with either should appear,
        // we don't require both
        //
        // TODO: If we set a scheduled date and the task has it, it must meet it (same for
        // deadlines), and if both are set it must meet both. *But*, if it doesn't have one of
        // them, that's fine, *unless* `force_match` is set.
        .filter(|t| {
            t.is_err()
                || t.as_ref().is_ok_and(|i| {
                    meets_dt(i.scheduled, args.scheduled, args.force_scheduled)
                        && meets_dt(i.deadline, args.deadline, args.force_deadline)
                        && (!args.force_match || i.scheduled.is_some() || i.deadline.is_some())
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let projects = action_items
        .values()
        .flat_map(|item| Project::from_action_item(item, &action_items))
        .filter(|p| {
            p.is_err()
                || p.as_ref().is_ok_and(|i| {
                    meets_dt(i.scheduled, args.scheduled, args.force_scheduled)
                        && meets_dt(i.deadline, args.deadline, args.force_deadline)
                        && (!args.force_match || i.scheduled.is_some() || i.deadline.is_some())
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let waitings = action_items
        .values()
        .flat_map(|item| Waiting::from_action_item(item, &action_items))
        .filter(|w| {
            w.is_err()
                || w.as_ref().is_ok_and(|i| {
                    meets_dt(i.scheduled, args.scheduled, args.force_scheduled)
                        && meets_dt(i.deadline, args.deadline, args.force_deadline)
                        && (!args.force_match || i.scheduled.is_some() || i.deadline.is_some())
                })
        })
        .collect::<Result<Vec<_>>>()?;

    // Insert events for each day where there's at least one daily note
    if args.daily_note_events {
        events.extend(DailyNote::notes_to_events(daily_notes.iter()));
    }

    // Compute crunch points when things might get very busy (only necessary if the user wants them)
    let crunch_points = if args.crunch_points {
        Some(
            tasks
                .iter()
                .fold(CrunchPoints::new(), CrunchPoints::accumulate),
        )
    } else {
        None
    };

    // Work out which contexts we'll need to enter to complete all the low/minimal-effort tasks by
    // the due date
    let target_contexts = if let Some(deadline) = args.target_contexts {
        let mut contexts = tasks
            .iter()
            .filter(|t| {
                t.effort <= Effort::Low
                    && t.deadline
                        .is_some_and(|d| d <= deadline.and_hms_opt(23, 59, 59).unwrap())
            })
            .flat_map(|t| t.contexts.iter().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        contexts.sort_unstable();
        Some(contexts)
    } else {
        None
    };

    // Now that we have the crunch points, filter the tasks completely
    let (easy_tasks, hard_tasks) = tasks
        .into_iter()
        // Either we allow non-actionable tasks, or this task must be actionable
        .filter(|t| args.next_tasks || t.can_start)
        // If we have contexts to filter by, we only want tasks that have at least one context, and
        // where we have all the contexts they require
        .filter(|t| {
            args.contexts.is_empty()
                || (t.contexts.iter().all(|c| args.contexts.contains(c)) && !t.contexts.is_empty())
        })
        .filter(|t| t.effort >= min_effort && t.effort <= max_effort)
        .filter(|t| t.priority >= min_priority && t.priority <= max_priority)
        .filter(|t| {
            args.people.is_empty()
                || (t
                    .people
                    .iter()
                    .all(|(_id, person)| args.people.contains(&person))
                    && !t.people.is_empty())
        })
        .partition(|t| t.effort <= Effort::Low);

    let mut final_data = FinalData {
        events: args.events.then_some(events),
        daily_notes: args.daily_notes.then_some(daily_notes),
        tickles: args.tickles.then_some(tickles),
        person_dates: args.dates.then_some(person_dates),
        waitings: args.waits.then_some(waitings),
        projects: args.projects.then_some(projects),
        easy_tasks: (args.tasks || args.easy_tasks).then_some(easy_tasks),
        hard_tasks: (args.tasks || args.hard_tasks).then_some(hard_tasks),
        crunch_points,
        target_contexts,
    };

    // Sort everything the user asked for (totally pointless to sort other things)
    if let Some(events) = final_data.events.as_mut() {
        events.sort_unstable_by_key(|ev| {
            (
                ev.timestamp.start.date,
                ev.timestamp.start.time,
                ev.title.clone(),
            )
        });
    }
    if let Some(daily_notes) = final_data.daily_notes.as_mut() {
        daily_notes.sort_unstable_by_key(|dn| (dn.date, dn.title.clone()));
    }
    if let Some(tickles) = final_data.tickles.as_mut() {
        tickles.sort_unstable_by_key(|t| (t.date, t.title.clone()));
    }
    if let Some(person_dates) = final_data.person_dates.as_mut() {
        person_dates.sort_unstable_by_key(|pd| (pd.notify_date, pd.date, pd.title.clone()));
    }
    if let Some(waitings) = final_data.waitings.as_mut() {
        waitings.sort_unstable_by_key(|w| {
            (
                w.scheduled.unwrap_or(w.deadline.unwrap_or(post_until)),
                w.deadline.unwrap_or(post_until),
                w.title.clone(),
            )
        });
    }
    if let Some(projects) = final_data.projects.as_mut() {
        projects.sort_unstable_by_key(|p| {
            (
                p.scheduled.unwrap_or(p.deadline.unwrap_or(post_until)),
                p.deadline.unwrap_or(post_until),
                p.priority,
                p.title.clone(),
            )
        });
    }
    if let Some(tasks) = final_data.easy_tasks.as_mut() {
        tasks.sort_unstable_by_key(|t| {
            (
                t.scheduled.unwrap_or(t.deadline.unwrap_or(post_until)),
                t.deadline.unwrap_or(post_until),
                t.priority,
                t.title.clone(),
            )
        });
    }
    if let Some(tasks) = final_data.hard_tasks.as_mut() {
        tasks.sort_unstable_by_key(|t| {
            (
                t.scheduled.unwrap_or(t.deadline.unwrap_or(post_until)),
                t.deadline.unwrap_or(post_until),
                t.priority,
                t.title.clone(),
            )
        });
    }

    if args.encoding.bincode {
        let bytes = bincode::serialize(&final_data)?;
        std::io::stdout().write_all(&bytes)?;
        std::io::stdout().flush()?;
    } else {
        println!("{}", serde_json::to_string(&final_data)?);
    }

    Ok(())
}

/// The final data we stream to the caller.
#[derive(Serialize)]
struct FinalData {
    events: Option<Vec<Event>>,
    daily_notes: Option<Vec<DailyNote>>,
    tickles: Option<Vec<Tickle>>,
    person_dates: Option<Vec<PersonDate>>,
    easy_tasks: Option<Vec<Task>>,
    hard_tasks: Option<Vec<Task>>,
    projects: Option<Vec<Project>>,
    waitings: Option<Vec<Waiting>>,
    crunch_points: Option<CrunchPoints>,
    target_contexts: Option<Vec<String>>,
}

/// Determines whether or not a date on an item meets an imposed cutoff (e.g. its deadline is
/// before the cutoff).
///
/// If the imposed date is not present, this will return true. If it is present and the item has a
/// date as well, it will return true if the item's date is before or on the cutoff. If the item
/// does *not* have a date, it will return true if `force_imposed` is false, and false if it is
/// true. That is, when `force_imposed` is set, items without dates will not be allowed, whereas
/// when it's not, they will be, provided they meet the cutoff.
fn meets_dt(
    item_dt: Option<NaiveDateTime>,
    imposed_date: Option<NaiveDate>,
    force_imposed: bool,
) -> bool {
    // imposed_date.is_none() || item_dt.is_none() || item_dt.unwrap().date() <= imposed_date.unwrap()

    imposed_date.is_none()
        || item_dt.map_or(!force_imposed, |item_dt| {
            item_dt.date() <= imposed_date.unwrap()
        })
}
