use chrono::{DateTime, Utc};
use prost_types::Timestamp;

/// The time of a timestamp, `None` for a missing or out of range one.
pub fn to_time(timestamp: Option<&Timestamp>) -> Option<DateTime<Utc>> {
    timestamp.and_then(|timestamp| {
        DateTime::from_timestamp(timestamp.seconds, u32::try_from(timestamp.nanos).ok()?)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_a_timestamp() {
        let timestamp = Timestamp {
            seconds: 1_705_000_000,
            nanos: 0,
        };

        assert_eq!(
            to_time(Some(&timestamp)),
            DateTime::from_timestamp(1_705_000_000, 0)
        );
    }

    #[test]
    fn a_missing_or_invalid_timestamp_has_no_time() {
        let out_of_range = Timestamp {
            seconds: i64::MAX,
            nanos: 0,
        };
        let negative_nanos = Timestamp {
            seconds: 0,
            nanos: -1,
        };

        assert_eq!(to_time(None), None);
        assert_eq!(to_time(Some(&out_of_range)), None);
        assert_eq!(to_time(Some(&negative_nanos)), None);
    }
}
