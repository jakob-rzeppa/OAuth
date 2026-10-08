//! Logging setup. The minimum level comes from `LOG_LEVEL` (see `Config::log_level`).

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

/// Parses a `LOG_LEVEL` value: `debug`, `info`, `warn` or `error`, in any case.
pub fn parse_level(value: &str) -> Result<LevelFilter, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "debug" => Ok(LevelFilter::DEBUG),
        "info" => Ok(LevelFilter::INFO),
        "warn" => Ok(LevelFilter::WARN),
        "error" => Ok(LevelFilter::ERROR),
        other => Err(format!("`{other}` is not one of debug, info, warn, error")),
    }
}

/// Which events are logged: this crate and `tower_http` (the request layer, see `main::app`) at
/// `level`; every other dependency (hyper, sqlx, reqwest, redis) is capped at `warn`, so `debug`
/// is not drowned in library internals.
fn filter(level: LevelFilter) -> Targets {
    let dependency_level = if level > LevelFilter::WARN {
        LevelFilter::WARN
    } else {
        level
    };

    Targets::new()
        .with_target(env!("CARGO_CRATE_NAME"), level)
        .with_target("tower_http", level)
        .with_default(dependency_level)
}

/// The subscriber `init` installs, writing through `writer`.
fn subscriber<W>(level: LevelFilter, writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    // The filter is a layer of its own, not `fmt::layer().with_filter(..)`: a per-layer filter keeps
    // thread-local state, and sqlx's `tracing::enabled!` checks for its own (filtered out) events
    // leave it stale, which swallows the next real event on that thread.
    tracing_subscriber::registry()
        .with(filter(level))
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
}

/// Installs the global subscriber.
pub fn init(level: LevelFilter) {
    subscriber(level, std::io::stdout).init();
}

#[cfg(test)]
pub(crate) mod testing {
    use std::{
        io,
        sync::{Arc, Mutex},
    };

    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::fmt::MakeWriter;

    /// Collects log output so tests can assert on what was (and was not) logged.
    #[derive(Clone, Default)]
    pub struct LogCapture(Arc<Mutex<Vec<u8>>>);

    impl LogCapture {
        /// Installs a DEBUG-level subscriber writing into this capture for the current thread,
        /// until the returned guard is dropped. `#[tokio::test]` runs on a single thread, so the
        /// guard also covers everything the test awaits.
        pub fn install(&self) -> DefaultGuard {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_writer(self.clone())
                .with_ansi(false)
                .finish();
            tracing::subscriber::set_default(subscriber)
        }

        pub fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogCapture {
        type Writer = LogCapture;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{testing::LogCapture, *};

    #[test]
    fn parses_every_level_case_insensitively_and_ignores_surrounding_whitespace() {
        assert_eq!(parse_level("debug"), Ok(LevelFilter::DEBUG));
        assert_eq!(parse_level("INFO"), Ok(LevelFilter::INFO));
        assert_eq!(parse_level("Warn"), Ok(LevelFilter::WARN));
        assert_eq!(parse_level(" error \n"), Ok(LevelFilter::ERROR));
    }

    #[test]
    fn rejects_everything_that_is_not_one_of_the_four_levels() {
        for value in ["", " ", "fatal", "trace", "warning", "verbose", "3"] {
            assert!(parse_level(value).is_err(), "`{value}` was accepted");
        }
    }

    #[test]
    fn capture_records_events_from_the_installing_thread() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        tracing::debug!(client_id = "abc", "hello");

        let log = capture.contents();
        assert!(log.contains("hello"), "{log}");
        assert!(log.contains("abc"), "{log}");
    }

    #[test]
    fn enables_this_crate_and_the_request_layer_at_the_level_and_caps_other_dependencies_at_warn() {
        let debug_filter = filter(LevelFilter::DEBUG);
        assert!(debug_filter.would_enable("auth_server::token", &tracing::Level::DEBUG));
        assert!(debug_filter.would_enable("tower_http::trace::on_request", &tracing::Level::DEBUG));
        assert!(
            !debug_filter.would_enable("hyper_util::client::legacy::pool", &tracing::Level::DEBUG)
        );
        assert!(debug_filter.would_enable("sqlx::query", &tracing::Level::WARN));

        let error_filter = filter(LevelFilter::ERROR);
        assert!(!error_filter.would_enable("sqlx::query", &tracing::Level::WARN));
        assert!(!error_filter.would_enable("auth_server::token", &tracing::Level::WARN));
        assert!(error_filter.would_enable("auth_server::token", &tracing::Level::ERROR));
    }

    #[test]
    fn does_not_drop_the_event_that_follows_a_filtered_out_enabled_check() {
        let capture = LogCapture::default();
        let dispatch = tracing::Dispatch::new(subscriber(LevelFilter::INFO, capture.clone()));

        tracing::dispatcher::with_default(&dispatch, || {
            for i in 0..20 {
                // What sqlx does around every query: ask whether a DEBUG event of its own would
                // be logged. The answer is no; that must not swallow the next real event.
                assert!(!tracing::enabled!(target: "sqlx::query", tracing::Level::DEBUG));
                tracing::info!("kept {i}");
            }
        });

        let log = capture.contents();
        assert_eq!(log.matches("kept").count(), 20, "{log}");
    }
}
