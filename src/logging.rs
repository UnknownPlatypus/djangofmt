use anyhow::Result;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_tree::time::Uptime;

#[derive(Debug, Default, PartialOrd, Ord, PartialEq, Eq, Copy, Clone)]
pub enum LogLevel {
    /// No output ([`log::LevelFilter::Off`]).
    Quiet,
    /// All user-facing output ([`log::LevelFilter::Info`]).
    #[default]
    Default,
    /// All outputs ([`log::LevelFilter::Debug`]).
    Verbose,
}

impl LogLevel {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    const fn level_filter(&self) -> LevelFilter {
        match self {
            LogLevel::Default => LevelFilter::INFO,
            LogLevel::Verbose => LevelFilter::DEBUG,
            LogLevel::Quiet => LevelFilter::OFF,
        }
    }
}

pub fn setup_tracing(level: LogLevel) -> Result<()> {
    let filter = level.level_filter();

    if level == LogLevel::Verbose {
        tracing_subscriber::registry()
            .with(
                tracing_tree::HierarchicalLayer::default()
                    .with_indent_lines(true)
                    .with_indent_amount(2)
                    .with_bracketed_fields(true)
                    .with_thread_ids(true)
                    .with_targets(true)
                    .with_writer(|| Box::new(std::io::stderr()))
                    .with_timer(Uptime::default())
                    .with_filter(filter),
            )
            .init()
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(format().compact().without_time().with_target(false))
                    .with_filter(filter),
            )
            .init()
    };

    Ok(())
}
