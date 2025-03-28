use super::{node::Node, ActionItemRepeat, SimpleTimestamp};
use chrono::{NaiveDate, NaiveTime};
use orgish::Timestamp;

/// Expands any timestamps on the given node, repeating them until `until`. This ensures that no
/// timestamp has repeaters. This expects to only see active timestamps (read: run
/// `prune_inactive_ts` first).
///
/// This will treat each "primary" timestamp (i.e. in the heading) as the guide timestamp, which
/// will control the repeating cadence. If there are multiple such timestamps, they will each be
/// handled separately. Regardless, this will return a vector of all the nodes produced.
pub fn expand_timestamps(
    node: &Node,
    until: NaiveDate,
) -> impl Iterator<Item = ActionItemRepeat> + '_ {
    // If we handle the two cases of having primary timestamps and not having primary timestamps
    // separately, then we get two different iterators whose types don't match. To avoid that, we
    // instead extract the timestamps and convert them all to `Some(..)`. Then, if there are none,
    // we'll insert a `None` (this will be the only ever `None`). Then we can iterate over *that*,
    // and handling the option is equivalent to handling both cases, in one iterator!
    let mut extracted_timestamps = Vec::new();
    for ts in &node.metadata.as_ref().unwrap().timestamps {
        extracted_timestamps.push(Some(ts.clone()));
    }
    if extracted_timestamps.is_empty() {
        extracted_timestamps.push(None);
    }

    extracted_timestamps.into_iter().flat_map(move |ts| {
        RepeatData {
            primary: ts, // If we have a timestamp, use it, otherwise there's no primary timestamp
            scheduled: node.metadata.as_ref().unwrap().scheduled.clone(),
            deadline: node.metadata.as_ref().unwrap().deadline.clone(),
        }
        .repeat_until(until)
    })
}

/// Interim data for a repeat.
struct RepeatData {
    /// The primary timestamp on the node, if there is one.
    primary: Option<Timestamp>,
    /// The scheduled timestamp on the node, if there is one.
    scheduled: Option<Timestamp>,
    /// The deadline timestamp on the node, if there is one.
    deadline: Option<Timestamp>,
}
impl RepeatData {
    /// Produces the next repeat from the given repeat data, if one exists.
    fn next_repeat(&self) -> Option<RepeatData> {
        let mut is_next_repeat = false;
        let mut next_repeat = RepeatData {
            primary: None,
            scheduled: None,
            deadline: None,
        };

        // If there's a repeating main timestamp, preserve that
        if self
            .primary
            .as_ref()
            .is_some_and(|ts| ts.repeater.is_some())
        {
            is_next_repeat = true;
            next_repeat.primary = Some(
                self.primary
                    .as_ref()
                    .unwrap()
                    .clone()
                    .into_next_repeat()
                    .unwrap(),
            );
        }
        if self
            .scheduled
            .as_ref()
            .is_some_and(|ts| ts.repeater.is_some())
        {
            is_next_repeat = true;
            next_repeat.scheduled = Some(
                self.scheduled
                    .as_ref()
                    .unwrap()
                    .clone()
                    .into_next_repeat()
                    .unwrap(),
            );
        }
        if self
            .deadline
            .as_ref()
            .is_some_and(|ts| ts.repeater.is_some())
        {
            is_next_repeat = true;
            next_repeat.deadline = Some(
                self.deadline
                    .as_ref()
                    .unwrap()
                    .clone()
                    .into_next_repeat()
                    .unwrap(),
            );
        }

        if is_next_repeat {
            Some(next_repeat)
        } else {
            None
        }
    }

    /// Returns whether or not any timestamp on this repeat data applies before the given date.
    fn has_ts_before(&self, date: NaiveDate) -> bool {
        if self
            .primary
            .as_ref()
            .is_some_and(|ts| ts.start.date <= date)
        {
            return true;
        }
        if self
            .scheduled
            .as_ref()
            .is_some_and(|ts| ts.start.date <= date)
        {
            return true;
        }
        if self
            .deadline
            .as_ref()
            .is_some_and(|ts| ts.start.date <= date)
        {
            return true;
        }

        false
    }

    /// Returns true if none of the timestamps in this repeat data are populated.
    fn is_empty(&self) -> bool {
        self.primary.is_none() && self.scheduled.is_none() && self.deadline.is_none()
    }

    /// Produces an iterator of individual repeat information packets until the given date, for
    /// this repeat data.
    fn repeat_until(self, until: NaiveDate) -> impl Iterator<Item = ActionItemRepeat> {
        let mut last_repeat_opt = Some(self);
        std::iter::from_fn(move || {
            if let Some(last_repeat) = last_repeat_opt.take() {
                // Get the next repeat, and save it to yield next time if any part of it falls
                // before the cutoff
                let next_repeat = last_repeat.next_repeat();
                if next_repeat.as_ref().is_some_and(|r| r.has_ts_before(until)) {
                    last_repeat_opt = next_repeat;
                }

                // Yield the last repeat, turning it into a static repeat (i.e. disconnecting the
                // information about how it repeats from when this single repeat actually falls).
                // We do this even if the repeat has no information (which would only be the case
                // once), to allow iterators over repeats to always produce something useful.
                //
                // We do make sure the first repeat is actually before the cutoff though!
                if !last_repeat.has_ts_before(until) && !last_repeat.is_empty() {
                    None
                } else {
                    Some(ActionItemRepeat {
                        primary: last_repeat.primary.map(|ts| SimpleTimestamp {
                            start: ts.start,
                            end: ts.end,
                        }),
                        scheduled: last_repeat.scheduled.map(|ts| {
                            ts.start.date.and_time(
                                ts.start
                                    .time
                                    .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
                            )
                        }),
                        deadline: last_repeat.deadline.map(|ts| {
                            ts.start.date.and_time(
                                ts.start
                                    .time
                                    .unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
                            )
                        }),
                    })
                }
            } else {
                None
            }
        })
    }
}
