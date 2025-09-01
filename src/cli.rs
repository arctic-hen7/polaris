use crate::views::{AllViews, View};
use anyhow::{bail, Context, Error};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::{collections::HashMap, ops::Deref, path::PathBuf, str::FromStr};

/// Polaris, the ultimate scheduling tool.
#[derive(Parser, Debug)]
pub struct Cli {
    #[command(flatten)]
    view_options: ViewOptions,

    /// Completion keywords to recognise and exclude from the action items.
    #[arg(long, default_values_t = vec!["DONE".to_string(), "CONT".to_string(), "PROB".to_string()])]
    pub done_keywords: Vec<String>,
    /// The address of the Starling endpoint from which to fetch action items.
    #[arg(long = "starling", default_value = "localhost:3000")]
    pub starling_address: String,
    /// Which encoding to output.
    #[arg(short, long, default_value = "json")]
    pub encoding: Encoding,
    /// The amount of time to add after the last date in the views to guide when to stop expanding
    /// repeating timestamps. If there are no date filters, this will be added to the present date.
    /// It should be large enough to account for the longest person date notification times in
    /// particular.
    #[arg(long, default_value = "8w")]
    pub repeat_buffer: RepeatBuffer,
}
impl Cli {
    /// Extracts the views from the options, which may involve reading a JSON definition of them.
    /// If the user has requested help on the views, this will return `Ok(None)`, and the caller
    /// should exit the process (help is printed automatically). This will group the views by data
    /// type, and work out the latest date among them.
    pub fn parse_views(&mut self) -> Result<Option<AllViews>, Error> {
        // First, get a vector of views, all with different data types
        let views_vec = if let Some(views_help) = &self.view_options.views_help {
            NamedView::try_parse_from(
                std::iter::once("polaris_view").chain(
                    views_help
                        .iter()
                        .map(String::as_str)
                        .chain(std::iter::once("--help")),
                ),
            )?;
            return Ok(None);
        } else if !self.view_options.views.is_empty() {
            Ok::<_, Error>(std::mem::take(&mut self.view_options.views))
        } else if let Some(json_path) = &self.view_options.views_json {
            let json_contents = std::fs::read_to_string(json_path)
                .with_context(|| "failed to read json views file")?;
            let views: HashMap<String, JsonView> = serde_json::from_str(&json_contents)
                .with_context(|| "failed to parse json views file")?;
            let views_vec = views
                .into_iter()
                .flat_map(|(name, view)| {
                    let vec = match view {
                        JsonView::Single(view) => vec![view],
                        JsonView::Multiple(v) => v,
                    };
                    vec.into_iter().map(move |view| NamedView {
                        name: name.clone(),
                        view,
                    })
                })
                .collect();
            Ok(views_vec)
        } else {
            // We're guaranteed to have one of them set by `clap`'s parsing rules
            unreachable!()
        }?;

        // Now organise them by data type
        let mut all_views = AllViews {
            events: Vec::new(),
            daily_notes: Vec::new(),
            tickles: Vec::new(),
            dates: Vec::new(),
            waits: Vec::new(),
            stacks: Vec::new(),
            tasks: Vec::new(),
            target_contexts: Vec::new(),
            #[cfg(feature = "goals")]
            goals: Vec::new(),

            last_date: None,
        };
        for named_view in views_vec {
            // Validate the view, which will also return the last date in it
            let last_date = named_view
                .view
                .validate()
                .with_context(|| format!("failed to validate view `{}`", named_view.name))?;

            // Add the view to the appropriate vector
            match named_view.view {
                View::Events(filter) => all_views.events.push((named_view.name, filter)),
                View::DailyNotes(filter) => all_views.daily_notes.push((named_view.name, filter)),
                View::Tickles(filter) => all_views.tickles.push((named_view.name, filter)),
                View::Dates(filter) => all_views.dates.push((named_view.name, filter)),
                View::Waits(filter) => all_views.waits.push((named_view.name, filter)),
                View::Stacks(filter) => all_views.stacks.push((named_view.name, filter)),
                View::Tasks(filter) => all_views.tasks.push((named_view.name, filter)),
                View::TargetContexts(filter) => {
                    all_views.target_contexts.push((named_view.name, filter))
                }
                #[cfg(feature = "goals")]
                View::Goals(filter) => all_views.goals.push((named_view.name, filter)),
            }

            // If we have a last date, update it
            if let Some(last_date) = last_date {
                if all_views
                    .last_date
                    .is_none_or(|latest_date| latest_date < last_date)
                {
                    all_views.last_date = Some(last_date);
                }
            }
        }

        // The `Ok(None)` branch was handled in the first section
        Ok(Some(all_views))
    }
}

/// Options that allow the user to pass views directly, with a JSON file (for more complex
/// configurations), or to get help around how to specify views.
#[derive(Parser, Debug)]
#[group(multiple = false, required = true)]
struct ViewOptions {
    /// Every one of these will create a new view (e.g. `--view "my_view events -u 2025-01-01"`).
    /// Within each argument, a separate CLI parse occurs, see help by running `polaris
    /// --help-views`
    #[arg(short, long = "view", num_args=1.., value_parser)]
    views: Vec<NamedView>,

    /// The path to a JSON file declaring the views to use as a map of view names to view options
    #[arg(short = 'j', long = "views-json")]
    views_json: Option<PathBuf>,

    /// Produces a help message about how to to specify views on the CLI (you can add a particular
    /// subcommand after this to get more detailed info)
    #[arg(long = "help-views", trailing_var_arg = true, num_args = 0..)]
    views_help: Option<Vec<String>>,
}

/// The encoding to use for the output of the CLI.
#[derive(ValueEnum, Clone, Debug)]
#[clap(rename_all = "snake_case")]
pub enum Encoding {
    /// JSON, the default encoding.
    Json,
    /// Bincode, which is *much* faster to handle if passing output to another Rust program.
    Bincode,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonView {
    Single(View),
    Multiple(Vec<View>),
}

/// A wrapper type over the duration buffer which will be added after the last date we detect
/// across all the views the user specifies. This allows accounting for things like long
/// notification times on person-related dates, which will only be detected if we expand timestamps
/// far enough into the future to generate the dates themselves (and then we can work backward to
/// their notification dates).
///
/// This also needs to be long enough to catch deadlines on non-actionable tasks so we can
/// potentially adjust those on actionable tasks within our window of concern accordingly.
#[derive(Clone, Debug)]
pub struct RepeatBuffer(pub chrono::Duration);
impl FromStr for RepeatBuffer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The last character will guide the amount of time
        let duration = match s.chars().last() {
            Some('w') => chrono::Duration::weeks(s[..s.len() - 1].parse()?),
            Some('d') => chrono::Duration::days(s[..s.len() - 1].parse()?),
            _ => bail!("invalid repeat buffer format, expected a number followed by 'w', 'd', 'h', 'm', or 's'"),
        };
        Ok(RepeatBuffer(duration))
    }
}
impl Deref for RepeatBuffer {
    type Target = chrono::Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A view with a name, which will be parsed from what is effectively a sub-CLI inside the
/// `-v/--view` argument.
#[derive(Parser, Clone, Debug)]
// #[command(disable_help_flag = true)]
pub struct NamedView {
    /// The name of the view to produce, which will be the key in the final output map.
    name: String,

    #[clap(subcommand)]
    view: View,
}
impl FromStr for NamedView {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // We split the user's input as a shell would (respecting quotes, that's the only thing
        // that can cause an error here), and then we parse it directly
        let parts = shellwords::split(s).map_err(|_| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                "mismatched quotes in view arguments",
            )
        })?;
        let fake_argv = std::iter::once("polaris_view").chain(parts.iter().map(|s| s.as_str()));
        NamedView::try_parse_from(fake_argv)
    }
}
