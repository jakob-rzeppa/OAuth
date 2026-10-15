use spec_api::spec_list_item::Kind;

use crate::{
    api::mapping::to_timestamp,
    application::spec_service::list::SpecOrDraftOverview,
    domain::projection::{
        draft_overview::DraftOverviewProjection, spec_overview::SpecOverviewProjection,
    },
};

impl From<SpecOverviewProjection> for spec_api::SpecOverview {
    fn from(spec: SpecOverviewProjection) -> Self {
        Self {
            id: spec.id.to_string(),
            title: spec.title,
            abbreviation: spec.abbreviation,
            created_by: spec.created_by.to_string(),
            created_at: Some(to_timestamp(spec.created_at)),
        }
    }
}

impl From<DraftOverviewProjection> for spec_api::DraftOverview {
    fn from(draft: DraftOverviewProjection) -> Self {
        Self {
            id: draft.id.to_string(),
            title: draft.title,
            abbreviation: draft.abbreviation,
            visibility: spec_api::DraftVisibility::from(draft.visibility).into(),
            current_version: draft.current_version,
            created_by: draft.created_by.to_string(),
            created_at: Some(to_timestamp(draft.created_at)),
        }
    }
}

impl From<SpecOrDraftOverview> for spec_api::SpecListItem {
    fn from(item: SpecOrDraftOverview) -> Self {
        let kind = match item {
            SpecOrDraftOverview::Spec(spec) => Kind::Published(spec.into()),
            SpecOrDraftOverview::Draft(draft) => Kind::Draft(draft.into()),
        };
        Self { kind: Some(kind) }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::{api::mapping::to_timestamp, domain::entity::spec::draft::DraftVisibility};

    fn spec_overview() -> SpecOverviewProjection {
        SpecOverviewProjection {
            id: Uuid::new_v4(),
            title: "Title".to_string(),
            abbreviation: "ABBR".to_string(),
            created_by: Uuid::new_v4(),
            created_at: Utc.timestamp_opt(1_700_000_000, 123).unwrap(),
        }
    }

    fn draft_overview() -> DraftOverviewProjection {
        DraftOverviewProjection {
            id: Uuid::new_v4(),
            title: "Draft".to_string(),
            abbreviation: "DRFT".to_string(),
            visibility: DraftVisibility::Hidden,
            current_version: 4,
            created_by: Uuid::new_v4(),
            created_at: Utc.timestamp_opt(1_700_000_000, 123).unwrap(),
        }
    }

    #[test]
    fn spec_overview_maps_every_field() {
        let overview = spec_overview();
        let (id, created_by, created_at) = (overview.id, overview.created_by, overview.created_at);

        let message = spec_api::SpecOverview::from(overview);

        assert_eq!(message.id, id.to_string());
        assert_eq!(message.title, "Title");
        assert_eq!(message.abbreviation, "ABBR");
        assert_eq!(message.created_by, created_by.to_string());
        assert_eq!(message.created_at, Some(to_timestamp(created_at)));
    }

    #[test]
    fn draft_overview_maps_every_field() {
        let overview = draft_overview();
        let (id, created_by, created_at) = (overview.id, overview.created_by, overview.created_at);

        let message = spec_api::DraftOverview::from(overview);

        assert_eq!(message.id, id.to_string());
        assert_eq!(message.title, "Draft");
        assert_eq!(message.abbreviation, "DRFT");
        assert_eq!(message.visibility, spec_api::DraftVisibility::Hidden as i32);
        assert_eq!(message.current_version, 4);
        assert_eq!(message.created_by, created_by.to_string());
        assert_eq!(message.created_at, Some(to_timestamp(created_at)));
    }

    #[test]
    fn a_spec_becomes_a_published_list_item() {
        let item = spec_api::SpecListItem::from(SpecOrDraftOverview::Spec(spec_overview()));

        assert!(matches!(
            item.kind,
            Some(spec_api::spec_list_item::Kind::Published(_))
        ));
    }

    #[test]
    fn a_draft_becomes_a_draft_list_item() {
        let item = spec_api::SpecListItem::from(SpecOrDraftOverview::Draft(draft_overview()));

        assert!(matches!(
            item.kind,
            Some(spec_api::spec_list_item::Kind::Draft(_))
        ));
    }
}
