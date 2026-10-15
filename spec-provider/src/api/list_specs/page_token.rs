//! The opaque `page_token` handed to clients: the position after which the next page starts.
//!
//! Clients must not parse tokens, so the format may change.

use chrono::DateTime;
use uuid::Uuid;

use crate::application::contract::PageCursor;

/// Encodes the cursor as `<seconds>:<nanos>:<uuid>`.
pub fn encode(cursor: &PageCursor) -> String {
    format!(
        "{}:{}:{}",
        cursor.created_at.timestamp(),
        cursor.created_at.timestamp_subsec_nanos(),
        cursor.id
    )
}

/// Returns `None` if the token is not one produced by `encode`.
pub fn decode(token: &str) -> Option<PageCursor> {
    let mut parts = token.split(':');
    let seconds = parts.next()?.parse().ok()?;
    let nanos = parts.next()?.parse().ok()?;
    let id = Uuid::parse_str(parts.next()?).ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some(PageCursor {
        created_at: DateTime::from_timestamp(seconds, nanos)?,
        id,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn a_decoded_token_gives_back_the_cursor() {
        let cursor = PageCursor {
            created_at: Utc.timestamp_opt(1_700_000_000, 123_456_789).unwrap(),
            id: Uuid::new_v4(),
        };

        assert_eq!(decode(&encode(&cursor)), Some(cursor));
    }

    #[test]
    fn keeps_timestamps_before_1970() {
        let cursor = PageCursor {
            created_at: Utc.timestamp_opt(-86_400, 999_999_999).unwrap(),
            id: Uuid::new_v4(),
        };

        assert_eq!(decode(&encode(&cursor)), Some(cursor));
    }

    #[test]
    fn rejects_text_that_is_not_a_token() {
        assert_eq!(decode("not-a-token"), None);
        assert_eq!(decode(""), None);
    }

    #[test]
    fn rejects_a_token_with_a_malformed_part() {
        let id = Uuid::new_v4();

        assert_eq!(decode(&format!("abc:0:{id}")), None);
        assert_eq!(decode(&format!("0:abc:{id}")), None);
        assert_eq!(decode("0:0:not-a-uuid"), None);
    }

    #[test]
    fn rejects_a_token_with_nanos_out_of_range() {
        assert_eq!(decode(&format!("0:2000000000:{}", Uuid::new_v4())), None);
    }

    #[test]
    fn rejects_a_token_with_extra_parts() {
        assert_eq!(decode(&format!("0:0:{}:1", Uuid::new_v4())), None);
    }
}
