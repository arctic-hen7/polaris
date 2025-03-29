use super::Task;
use crate::Effort;
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;

/// Information about "crunch points", where multiple deadlines fall. This system uses the effort
/// levels of tasks due on a particualr date to ascribe a score to that date, which can be used to
/// estimate in advance when busy periods are upcoming.
#[derive(Serialize)]
pub struct CrunchPoints {
    pub inner: HashMap<NaiveDate, u32>,
}
impl CrunchPoints {
    /// Creates a new, empty set of crunch points.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Accumulates the details of the given task into this set of crunch points.
    pub fn accumulate(mut self, task: &Task) -> Self {
        if let Some(deadline) = task.deadline {
            *self.inner.entry(deadline.date()).or_insert(0) += effort_to_numeric(&task.effort);
        }

        self
    }
}

/// Converts the given effort level to a numeric value for crunch point estimation.
// TODO: Very crude and arbitrary right now...
fn effort_to_numeric(effort: &Effort) -> u32 {
    match effort {
        Effort::Total => 10,
        Effort::High => 4,
        Effort::Medium => 2,
        Effort::Low => 1,
        Effort::Minimal => 0,
    }
}
