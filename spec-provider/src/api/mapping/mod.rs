//! Conversions from domain projections into the gRPC messages of `spec_api`.

use chrono::{DateTime, Utc};

mod draft;
mod overview;
mod patch;
mod proposal;
mod spec;
mod version;

fn to_timestamp(date_time: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: date_time.timestamp(),
        nanos: date_time.timestamp_subsec_nanos() as i32,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn to_timestamp_keeps_seconds_and_nanos() {
        let date_time = Utc.timestamp_opt(1_700_000_000, 123_456_789).unwrap();

        let timestamp = to_timestamp(date_time);

        assert_eq!(timestamp.seconds, 1_700_000_000);
        assert_eq!(timestamp.nanos, 123_456_789);
    }
}
