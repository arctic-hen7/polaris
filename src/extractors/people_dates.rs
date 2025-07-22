use std::collections::HashMap;

use crate::ActionItem;
use anyhow::{anyhow, bail, Context, Result};
use chrono::{Duration, NaiveDate};
use serde::Serialize;
use uuid::Uuid;

/// A date associated with a person (e.g. a birthday or anniversary).
#[derive(Serialize, Clone, Debug)]
pub struct PersonDate {
    /// The unique ID of the node corresponding to this date.
    pub id: Uuid,
    /// The title of the date (e.g. birthday).
    pub title: String,
    /// The ID and name of the person this date is associated with.
    pub person: (Uuid, String),
    /// The body of the date, if there is one.
    pub body: Option<String>,
    /// The date itself (repeaters are obviously critical here, and have been handled by the
    /// initial fetching and parsing system).
    pub date: NaiveDate,
    /// The date on which we should be alerted that this date is coming up.
    pub notify_date: NaiveDate,
}
impl PersonDate {
    /// Converts the given action item into a person date, if its repeats would go in the person
    /// dates list.
    pub fn from_action_item<'a, 'm: 'a>(
        item: &'a ActionItem,
        _map: &'m HashMap<Uuid, ActionItem>,
    ) -> impl Iterator<Item = Result<Self>> + 'a {
        item.base().repeats.iter().filter_map(move |repeat| {
            if item.base().parent_tags.contains("person_dates") {
                if let ActionItem::None { properties, people, .. } = item {
                    repeat.primary.as_ref().map(|ts| {
                        if ts.end.is_some() || ts.start.time.is_some() {
                            bail!(
                                "person date {} is not an all-day event",
                                item.base().id
                            );
                        }
                        let date = ts.start.date;

                        // The `ADVANCE` property is of the form `nX`, where `n` is a number and
                        // `X` is a specifier. `X` can be either `d` for days or `w` for weeks. We
                        // parse this and use it to determine the notification date.
                        if let Some(advance) = properties.get("ADVANCE") {
                            let specifier = advance.chars().last().unwrap();
                            let number: u16 = advance[..advance.len() - 1]
                                .parse()
                                .with_context(|| format!("failed to parse ADVANCE for person date {}", item.base().id))?;
                            let notify_date = match specifier {
                                'd' => date - Duration::days(number as i64),
                                'w' => date - Duration::weeks(number as i64),
                                _ => bail!("invalid specifier in ADVANCE for person date {}", item.base().id),
                            };

                            // Parse the people to determine the person this date is associated with
                            let person = people
                            .iter()
                            .next()
                            .ok_or_else(|| anyhow!("person date {} must have a person they're associated with listed in PEOPLE", item.base().id))?;

                            Ok(Self {
                                id: item.base().id,
                                title: item.base().title.last().cloned().unwrap(),
                                body: item.base().body.clone(),
                                date,
                                notify_date,
                                person: person.clone(),
                            })
                        } else {
                            Err(anyhow!("person date {} must have an ADVANCE property", item.base().id))
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}
