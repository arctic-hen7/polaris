use super::GoalsSource;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use uuid::Uuid;

/// Returns the goal types for the given date. Specifically, this can return as many "types" of
/// goals as needed, with human-readable names (e.g. *Daily Goals*, *Weekly Goals*), and then
/// reference where these goals will be found using [`GoalsSource`] (see `mod.rs`).
///
/// You will almost certainly need to modify this function to get goal extraction working for your
/// personal goals setup, which tends to vary massively between people. It's currently configured
/// for my personal system, where each day has a `$ACE_JOURNALS_DIR/<year>/<month>/<day>.md` file
/// containing a *Goals for Tomorrow* heading, and a *Goals for Next Week* file if it's a Sunday. I
/// also use a "daily surfaces" system which shows the same goals every day.
pub(super) fn goals_for_date(date: NaiveDate) -> Vec<(String, GoalsSource)> {
    // Get the last Sunday (which will contain the relevant weekly goals), but if today is a
    // Sunday, those goals won't have been written yet, so use the previous Sunday!
    let last_sunday = if date.weekday() == Weekday::Sun {
        date - Duration::days(7)
    } else {
        date - Duration::days(date.weekday().num_days_from_sunday() as i64)
    };

    let daily_journal_file_path = format!(
        "journals/{}/{:02}/{:02}.md",
        date.year(),
        date.month(),
        date.day()
    );
    let weekly_journal_file_path = format!(
        "journals/{}/{:02}/{:02}.md",
        last_sunday.year(),
        last_sunday.month(),
        last_sunday.day()
    );

    let mut goals_sources = Vec::new();
    goals_sources.push((
        "Daily Goals".to_string(),
        GoalsSource::File {
            path: daily_journal_file_path.clone(),
            heading_path: vec!["Goals for Tomorrow".to_string()],
            fail_on_missing_heading: true,
        },
    ));
    goals_sources.push((
        "Weekly Goals".to_string(),
        GoalsSource::File {
            path: weekly_journal_file_path.clone(),
            heading_path: vec!["Goals for Next Week".to_string()],
            fail_on_missing_heading: true,
        },
    ));
    goals_sources.push((
        "Daily Surfaces".to_string(),
        GoalsSource::Id(Uuid::parse_str("9a73deb2-e702-47d0-8967-dc82de424237").unwrap()),
    ));

    goals_sources
}
