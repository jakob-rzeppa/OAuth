use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::spec::draft::{Draft, DraftVisibility};

/// The summary of a draft shown in listings.
pub struct DraftOverviewProjection {
    pub id: Uuid,

    pub title: String,
    pub abbreviation: String,

    pub visibility: DraftVisibility,

    /// The number of the latest cut version, which is the number of cut versions.
    /// `0` while no version has been cut.
    pub current_version: u32,

    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<&Draft> for DraftOverviewProjection {
    fn from(draft: &Draft) -> Self {
        Self {
            id: *draft.id(),
            title: draft.title().to_string(),
            abbreviation: draft.abbreviation().to_string(),
            visibility: *draft.visibility(),
            current_version: draft.versions().len() as u32,
            created_by: *draft.created_by(),
            created_at: *draft.created_at(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::domain::entity::version::Version;

    fn make_draft(versions: Vec<Version>) -> Draft {
        Draft::reconstitute(
            Uuid::new_v4(),
            "Title".to_string(),
            "ABBR".to_string(),
            DraftVisibility::Hidden,
            versions,
            Default::default(),
            Default::default(),
            Default::default(),
            Uuid::new_v4(),
            Utc.timestamp_opt(1_700_000_000, 5).unwrap(),
        )
    }

    #[test]
    fn copies_the_overview_fields_of_the_draft() {
        let draft = make_draft(vec![]);

        let overview = DraftOverviewProjection::from(&draft);

        assert_eq!(overview.id, *draft.id());
        assert_eq!(overview.title, "Title");
        assert_eq!(overview.abbreviation, "ABBR");
        assert!(matches!(overview.visibility, DraftVisibility::Hidden));
        assert_eq!(overview.created_by, *draft.created_by());
        assert_eq!(overview.created_at, *draft.created_at());
    }

    #[test]
    fn current_version_is_zero_without_cut_versions() {
        let overview = DraftOverviewProjection::from(&make_draft(vec![]));

        assert_eq!(overview.current_version, 0);
    }

    #[test]
    fn current_version_is_the_number_of_cut_versions() {
        let versions = vec![
            Version::new(vec![]),
            Version::new(vec![]),
            Version::new(vec![]),
        ];

        let overview = DraftOverviewProjection::from(&make_draft(versions));

        assert_eq!(overview.current_version, 3);
    }
}
