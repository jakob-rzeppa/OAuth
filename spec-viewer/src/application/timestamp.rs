use chrono::DateTime;
use prost_types::Timestamp;

/// Shown for a missing or out of range timestamp.
const UNKNOWN: &str = "unknown";

/// Formats as `2024-01-11`.
pub fn format_date(timestamp: Option<&Timestamp>) -> String {
    format(timestamp, "%Y-%m-%d")
}

/// Formats as `2024-01-11 18:13 UTC`.
pub fn format_date_time(timestamp: Option<&Timestamp>) -> String {
    format(timestamp, "%Y-%m-%d %H:%M UTC")
}

fn format(timestamp: Option<&Timestamp>, pattern: &str) -> String {
    timestamp
        .and_then(|timestamp| DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32))
        .map_or_else(
            || UNKNOWN.to_string(),
            |time| time.format(pattern).to_string(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMESTAMP: Timestamp = Timestamp {
        seconds: 1_705_000_000,
        nanos: 0,
    };

    #[test]
    fn formats_in_utc() {
        assert_eq!(format_date(Some(&TIMESTAMP)), "2024-01-11");
        assert_eq!(format_date_time(Some(&TIMESTAMP)), "2024-01-11 19:06 UTC");
    }

    #[test]
    fn formats_a_missing_or_invalid_timestamp_as_unknown() {
        let invalid = Timestamp {
            seconds: i64::MAX,
            nanos: 0,
        };

        assert_eq!(format_date(None), UNKNOWN);
        assert_eq!(format_date_time(Some(&invalid)), UNKNOWN);
    }
}
