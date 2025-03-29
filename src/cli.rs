use chrono::NaiveDate;
use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// The date range over which action items should be shown; if one value, it's interpreted as
    /// `until`, if two, it's `from` and `until` (a starting date only affects events and daily
    /// notes, and neither date affects tasks, projects, and waiting items)
    #[arg(value_names = &["from", "until"], num_args=1..=2, required = true)]
    pub date_range: Vec<NaiveDate>,

    /// Include events in the output
    #[arg(short, long)]
    pub events: bool,
    /// Disables daily note events in the output
    #[arg(long)]
    pub no_daily_note_events: bool,
    /// Include daily notes in the output
    #[arg(short = 'n', long)]
    pub daily_notes: bool,
    /// Include tickles in the output
    #[arg(short = 'i', long)]
    pub tickles: bool,
    /// Include important dates for people in the output
    #[arg(short, long)]
    pub dates: bool,
    /// Include waiting-for items in the output
    #[arg(short, long)]
    pub waits: bool,
    /// Include projects in the output
    #[arg(short, long)]
    pub projects: bool,
    /// Include tasks in the output
    #[arg(short, long)]
    pub tasks: bool,
    /// Include non-actionable tasks in the output
    #[arg(long)]
    pub next_tasks: bool,
    /// Include crunch points in the output
    #[arg(long)]
    pub crunch_points: bool,
    /// Produce the contexts which will need to be entered to complete all low/minimal-effort tasks
    /// due up until the given date
    #[arg(long)]
    pub target_contexts: Option<NaiveDate>,

    /// The last scheduled date to show (usually the present day, though to show all tasks which
    /// will appear over a week, set it to the end of the week; only affects tasks, projects, and
    /// waiting items)
    #[arg(long)]
    pub scheduled: Option<NaiveDate>,
    /// The last deadline date to show (only affects tasks, projects, and waiting items)
    #[arg(long)]
    pub deadline: Option<NaiveDate>,
    /// Force matches for scheduled/deadline dates (i.e. items without them won't be shown at all)
    #[arg(long)]
    pub force_match: bool,

    /// The contexts we have, which will filter to only tasks where all their required contexts are
    /// present (tasks with no contexts will not be shown here)
    #[arg(short, long)]
    pub contexts: Vec<String>,
    /// The minimum level of effort to show for tasks
    #[arg(long)]
    pub min_effort: Option<String>,
    /// The maximum level of effort to show for tasks
    #[arg(long)]
    pub max_effort: Option<String>,
    /// People to filter by for tasks (showing only tasks where all their required people are
    /// available, tasks with no people will not be shown here)
    #[arg(long = "person")]
    pub people: Vec<String>,

    /// The completion keywords to use
    #[arg(long, default_values_t = vec!["DONE".to_string(), "CONT".to_string(), "PROB".to_string()])]
    pub done_keywords: Vec<String>,
    /// The address of the Starling endpoint to use (e.g. `localhost:3000`)
    #[arg(long, default_value = "localhost:3000")]
    pub starling: String,

    #[command(flatten)]
    pub encoding: EncodingOptions,
}

#[derive(Parser)]
#[group(required = true, multiple = false)]
pub struct EncodingOptions {
    /// Encode the result as JSON
    #[arg(long)]
    pub json: bool,
    /// Encode the result with bincode (for other Rust programs)
    #[arg(long)]
    pub bincode: bool,
}
