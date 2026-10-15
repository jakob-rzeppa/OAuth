use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::spec::Spec;

/// The summary of a published spec shown in listings.
pub struct SpecOverviewProjection {
    pub id: Uuid,

    pub title: String,
    pub abbreviation: String,

    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<&Spec> for SpecOverviewProjection {
    fn from(spec: &Spec) -> Self {
        Self {
            id: *spec.id(),
            title: spec.title().to_string(),
            abbreviation: spec.abbreviation().to_string(),
            created_by: *spec.created_by(),
            created_at: *spec.created_at(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn copies_the_overview_fields_of_the_spec() {
        let id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let created_at = Utc.timestamp_opt(1_700_000_000, 5).unwrap();
        let spec = Spec::reconstitute(
            id,
            "Title".to_string(),
            "ABBR".to_string(),
            Default::default(),
            Default::default(),
            created_by,
            created_at,
        );

        let overview = SpecOverviewProjection::from(&spec);

        assert_eq!(overview.id, id);
        assert_eq!(overview.title, "Title");
        assert_eq!(overview.abbreviation, "ABBR");
        assert_eq!(overview.created_by, created_by);
        assert_eq!(overview.created_at, created_at);
    }
}
