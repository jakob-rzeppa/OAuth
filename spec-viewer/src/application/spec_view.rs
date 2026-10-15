use std::sync::Arc;

use chrono::{DateTime, Utc};
use spec_api::{Patch, Proposal, Version, get_spec_response::Kind};

use crate::application::{
    contract::{
        content_gateway::{ContentGateway, GetContentError},
        spec_gateway::{SpecGateway, SpecGatewayError},
    },
    diff::{Diff, diff},
    markdown::render_markdown,
    spec_status::{ProposalState, SpecStatus},
    timestamp::to_time,
};

/// A published spec or a draft with everything the detail page shows besides its text.
#[derive(Debug)]
pub struct SpecDetail {
    pub id: String,
    pub title: String,
    pub abbreviation: String,
    pub status: SpecStatus,
    /// The first version first.
    pub versions: Vec<VersionInfo>,
    /// The patches applied to the next version when it is cut. Always empty for published specs.
    pub unreleased_patches: Vec<PatchInfo>,
    /// Always empty for published specs.
    pub proposals: Vec<ProposalInfo>,
}

#[derive(Debug)]
pub struct VersionInfo {
    /// 1-based.
    pub number: usize,
    pub patches: Vec<PatchInfo>,
}

#[derive(Debug)]
pub struct PatchInfo {
    pub proposed_by: String,
    pub created_at: Option<DateTime<Utc>>,
    pub accepted_by: String,
    pub accepted_at: Option<DateTime<Utc>>,
    /// Where the full text of the spec with this patch applied is stored.
    pub content_uri: String,
}

#[derive(Debug)]
pub struct ProposalInfo {
    pub id: String,
    pub proposed_by: String,
    pub created_at: Option<DateTime<Utc>>,
    pub last_updated_at: Option<DateTime<Utc>>,
    pub status: ProposalState,
    /// Where the full text of the spec with this proposal applied is stored.
    pub content_uri: String,
}

/// What the text of the detail page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The 1-based version number.
    Version(usize),
    /// The latest version of a draft with its unreleased patches applied.
    Unreleased,
    /// The id of a proposal of a draft.
    Proposal(String),
    /// A single accepted patch, with the full text of the spec as it was applied.
    Patch {
        scope: PatchScope,
        /// 1-based within the scope.
        number: usize,
    },
}

/// Where a patch belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchScope {
    /// The 1-based version number.
    Version(usize),
    Unreleased,
}

impl Selection {
    /// The version or the unreleased changes this selection is part of, `None` for proposals.
    pub fn scope(&self) -> Option<PatchScope> {
        match self {
            Selection::Version(number) => Some(PatchScope::Version(*number)),
            Selection::Unreleased => Some(PatchScope::Unreleased),
            Selection::Patch { scope, .. } => Some(*scope),
            Selection::Proposal(_) => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// The text as rendered markdown.
    #[default]
    Rendered,
    /// The changes against the text before, like `git diff`.
    Diff,
}

#[derive(Debug, thiserror::Error)]
pub enum ViewSpecError {
    #[error("no such spec, version or proposal")]
    NotFound,
    #[error(transparent)]
    Provider(SpecGatewayError),
}

/// A spec as the detail page shows it.
#[derive(Debug)]
pub struct SpecView {
    pub spec: SpecDetail,
    /// `None` when the spec has no text yet.
    pub selection: Option<Selection>,
    pub mode: ViewMode,
    pub body: Body,
}

#[derive(Debug)]
pub enum Body {
    /// The spec has no text yet.
    Empty,
    /// Sanitized HTML.
    Rendered(String),
    Diff {
        /// What the selection is compared with, `None` for an empty text.
        base: Option<Selection>,
        diff: Diff,
    },
    /// The text could not be loaded from the content storage.
    Unavailable,
}

/// The use case of showing a spec with the text of one of its versions or proposals.
pub struct SpecViewService {
    spec_gateway: Arc<dyn SpecGateway>,
    content_gateway: Arc<dyn ContentGateway>,
}

impl SpecViewService {
    pub fn new(
        spec_gateway: Arc<dyn SpecGateway>,
        content_gateway: Arc<dyn ContentGateway>,
    ) -> Self {
        SpecViewService {
            spec_gateway,
            content_gateway,
        }
    }

    /// Loads the spec with the given id and the text of the selection.
    ///
    /// Without a selection, the latest version is shown. When the text can not be loaded the
    /// view is still returned, with [`Body::Unavailable`].
    pub async fn view_spec(
        &self,
        id: &str,
        selection: Option<Selection>,
        mode: ViewMode,
    ) -> Result<SpecView, ViewSpecError> {
        let spec = match self.spec_gateway.get_spec(id).await {
            Ok(kind) => SpecDetail::from(kind),
            // An id that is not even well formed is as unknown as one that doesn't exist.
            Err(SpecGatewayError::Status(status))
                if matches!(
                    status.code(),
                    tonic::Code::NotFound | tonic::Code::InvalidArgument
                ) =>
            {
                return Err(ViewSpecError::NotFound);
            }
            Err(error) => return Err(ViewSpecError::Provider(error)),
        };

        let selection = match selection {
            Some(selection) if spec.contains(&selection) => Some(selection),
            Some(_) => return Err(ViewSpecError::NotFound),
            None => spec.default_selection(),
        };

        let body = match &selection {
            Some(selection) => self.load_body(&spec, selection, mode).await,
            None => Body::Empty,
        };

        Ok(SpecView {
            spec,
            selection,
            mode,
            body,
        })
    }

    async fn load_body(&self, spec: &SpecDetail, selection: &Selection, mode: ViewMode) -> Body {
        let body = match mode {
            ViewMode::Rendered => self
                .text_at(spec.content_uri(selection))
                .await
                .map(|text| Body::Rendered(render_markdown(&text))),
            ViewMode::Diff => {
                let base = spec.base_of(selection);
                let (old, new) = tokio::join!(
                    self.text_at(base.as_ref().and_then(|base| spec.content_uri(base))),
                    self.text_at(spec.content_uri(selection)),
                );
                old.and_then(|old| {
                    Ok(Body::Diff {
                        base,
                        diff: diff(&old, &new?),
                    })
                })
            }
        };

        body.unwrap_or_else(|error| {
            eprintln!("failed to load the text of spec {}: {error}", spec.id);
            Body::Unavailable
        })
    }

    /// The text at `uri`, or an empty text without any.
    async fn text_at(&self, uri: Option<&str>) -> Result<String, GetContentError> {
        match uri {
            Some(uri) => self.content_gateway.get(uri).await,
            None => Ok(String::new()),
        }
    }
}

impl SpecDetail {
    pub fn is_draft(&self) -> bool {
        self.status.is_draft()
    }

    pub fn contains(&self, selection: &Selection) -> bool {
        match selection {
            Selection::Version(number) => (1..=self.versions.len()).contains(number),
            Selection::Unreleased => !self.unreleased_patches.is_empty(),
            Selection::Proposal(id) => self.proposal(id).is_some(),
            Selection::Patch { scope, number } => {
                (1..=self.patches_of(*scope).len()).contains(number)
            }
        }
    }

    /// The patches of a scope, none for a version that doesn't exist.
    pub fn patches_of(&self, scope: PatchScope) -> &[PatchInfo] {
        match scope {
            PatchScope::Version(number) => number
                .checked_sub(1)
                .and_then(|index| self.versions.get(index))
                .map_or(&[], |version| &version.patches),
            PatchScope::Unreleased => &self.unreleased_patches,
        }
    }

    /// The last patch of the newest of the first `count` versions that has any.
    fn last_patch_up_to(&self, count: usize) -> Option<&PatchInfo> {
        self.versions
            .iter()
            .take(count)
            .rev()
            .find_map(|version| version.patches.last())
    }

    pub fn proposal(&self, id: &str) -> Option<&ProposalInfo> {
        self.proposals.iter().find(|proposal| proposal.id == id)
    }

    fn latest_version(&self) -> Option<Selection> {
        Some(self.versions.len())
            .filter(|&count| count > 0)
            .map(Selection::Version)
    }

    /// The latest version, or the unreleased changes of a draft without versions.
    fn default_selection(&self) -> Option<Selection> {
        self.latest_version().or_else(|| {
            self.contains(&Selection::Unreleased)
                .then_some(Selection::Unreleased)
        })
    }

    /// The text a proposal would change: the unreleased changes, or the latest version.
    fn head(&self) -> Option<Selection> {
        if self.unreleased_patches.is_empty() {
            self.latest_version()
        } else {
            Some(Selection::Unreleased)
        }
    }

    /// What the diff of the selection compares with, `None` for an empty text.
    fn base_of(&self, selection: &Selection) -> Option<Selection> {
        match selection {
            Selection::Version(number) => number
                .checked_sub(1)
                .filter(|&previous| previous > 0)
                .map(Selection::Version),
            Selection::Unreleased => self.latest_version(),
            Selection::Proposal(_) => self.head(),
            Selection::Patch { scope, number } if *number > 1 => Some(Selection::Patch {
                scope: *scope,
                number: number - 1,
            }),
            // The first patch changes what the patches before it left.
            Selection::Patch { scope, .. } => {
                let before = match scope {
                    PatchScope::Version(number) => number.saturating_sub(1),
                    PatchScope::Unreleased => self.versions.len(),
                };
                self.versions
                    .iter()
                    .take(before)
                    .enumerate()
                    .rev()
                    .find(|(_, version)| !version.patches.is_empty())
                    .map(|(index, version)| Selection::Patch {
                        scope: PatchScope::Version(index + 1),
                        number: version.patches.len(),
                    })
            }
        }
    }

    /// Every patch stores the full text, so the text of a selection is the one of its last
    /// patch. `None` when there is no patch yet.
    fn content_uri(&self, selection: &Selection) -> Option<&str> {
        match selection {
            Selection::Version(number) if *number <= self.versions.len() => {
                self.last_patch_up_to(*number)
            }
            Selection::Version(_) => None,
            Selection::Unreleased => self
                .unreleased_patches
                .last()
                .or_else(|| self.last_patch_up_to(self.versions.len())),
            Selection::Patch { scope, number } => number
                .checked_sub(1)
                .and_then(|index| self.patches_of(*scope).get(index)),
            Selection::Proposal(id) => {
                return self
                    .proposal(id)
                    .map(|proposal| proposal.content_uri.as_str());
            }
        }
        .map(|patch| patch.content_uri.as_str())
    }
}

impl From<Kind> for SpecDetail {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Published(spec) => SpecDetail {
                id: spec.id,
                title: spec.title,
                abbreviation: spec.abbreviation,
                status: SpecStatus::Published,
                versions: versions(spec.versions),
                unreleased_patches: Vec::new(),
                proposals: Vec::new(),
            },
            Kind::Draft(draft) => SpecDetail {
                status: SpecStatus::Draft {
                    visibility: draft.visibility().into(),
                },
                id: draft.id,
                title: draft.title,
                abbreviation: draft.abbreviation,
                versions: versions(draft.versions),
                unreleased_patches: draft
                    .unversioned_patches
                    .into_iter()
                    .map(PatchInfo::from)
                    .collect(),
                proposals: draft
                    .proposals
                    .into_iter()
                    .map(ProposalInfo::from)
                    .collect(),
            },
        }
    }
}

fn versions(versions: Vec<Version>) -> Vec<VersionInfo> {
    versions
        .into_iter()
        .enumerate()
        .map(|(index, version)| VersionInfo {
            number: index + 1,
            patches: version.patches.into_iter().map(PatchInfo::from).collect(),
        })
        .collect()
}

impl From<Patch> for PatchInfo {
    fn from(patch: Patch) -> Self {
        PatchInfo {
            created_at: to_time(patch.created_at.as_ref()),
            accepted_at: to_time(patch.accepted_at.as_ref()),
            proposed_by: patch.proposed_by,
            accepted_by: patch.accepted_by,
            content_uri: patch.content_uri,
        }
    }
}

impl From<Proposal> for ProposalInfo {
    fn from(proposal: Proposal) -> Self {
        ProposalInfo {
            created_at: to_time(proposal.created_at.as_ref()),
            last_updated_at: to_time(proposal.last_updated_at.as_ref()),
            status: proposal.status().into(),
            id: proposal.id,
            proposed_by: proposal.proposed_by,
            content_uri: proposal.content_uri,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::contract::{
        content_gateway::MockContentGateway, spec_gateway::MockSpecGateway,
    };
    use crate::application::spec_status::Visibility;
    use spec_api::{Draft, DraftVisibility, ProposalStatus, Spec};

    fn patch(content_uri: &str) -> Patch {
        Patch {
            id: content_uri.into(),
            content_uri: content_uri.into(),
            ..Default::default()
        }
    }

    fn version(patches: &[&str]) -> Version {
        Version {
            id: String::new(),
            patches: patches.iter().map(|uri| patch(uri)).collect(),
        }
    }

    /// Version 1 with `v1a` and `v1b`, an empty version 2, unreleased `u` and proposal `p`.
    fn draft() -> SpecDetail {
        SpecDetail::from(Kind::Draft(Draft {
            id: "draft".into(),
            visibility: DraftVisibility::Public.into(),
            versions: vec![version(&["v1a", "v1b"]), version(&[])],
            unversioned_patches: vec![patch("u")],
            proposals: vec![Proposal {
                id: "p".into(),
                proposed_by: "alice".into(),
                status: ProposalStatus::Ready.into(),
                content_uri: "proposal".into(),
                ..Default::default()
            }],
            ..Default::default()
        }))
    }

    fn published(versions: Vec<Version>) -> SpecDetail {
        SpecDetail::from(Kind::Published(Spec {
            id: "spec".into(),
            versions,
            ..Default::default()
        }))
    }

    #[test]
    fn shows_every_patch_of_a_version() {
        let spec = draft();
        let patch = |scope, number| Selection::Patch { scope, number };

        assert!(spec.contains(&patch(PatchScope::Version(1), 2)));
        assert!(!spec.contains(&patch(PatchScope::Version(1), 0)));
        assert!(!spec.contains(&patch(PatchScope::Version(1), 3)));
        // Version 2 has no patches.
        assert!(!spec.contains(&patch(PatchScope::Version(2), 1)));
        assert!(!spec.contains(&patch(PatchScope::Version(3), 1)));
        assert!(spec.contains(&patch(PatchScope::Unreleased, 1)));

        assert_eq!(
            spec.content_uri(&patch(PatchScope::Version(1), 1)),
            Some("v1a")
        );
        assert_eq!(
            spec.content_uri(&patch(PatchScope::Version(1), 2)),
            Some("v1b")
        );
    }

    #[test]
    fn diffs_a_patch_against_the_patch_before() {
        let spec = draft();
        let patch = |scope, number| Selection::Patch { scope, number };

        assert_eq!(spec.base_of(&patch(PatchScope::Version(1), 1)), None);
        assert_eq!(
            spec.base_of(&patch(PatchScope::Version(1), 2)),
            Some(patch(PatchScope::Version(1), 1))
        );
        // Version 2 is empty, so the unreleased patch follows version 1.
        assert_eq!(
            spec.base_of(&patch(PatchScope::Unreleased, 1)),
            Some(patch(PatchScope::Version(1), 2))
        );
    }

    #[test]
    fn never_panics_for_a_selection_that_does_not_exist() {
        let spec = draft();
        let selections = [
            Selection::Version(0),
            Selection::Version(3),
            Selection::Version(usize::MAX),
            Selection::Proposal("missing".into()),
        ]
        .into_iter()
        .chain([0, 1, 3, usize::MAX].into_iter().flat_map(|number| {
            [0, 1, 3, usize::MAX]
                .into_iter()
                .map(PatchScope::Version)
                .chain([PatchScope::Unreleased])
                .map(move |scope| Selection::Patch { scope, number })
        }));

        for selection in selections {
            if !spec.contains(&selection) {
                assert_eq!(spec.content_uri(&selection), None, "{selection:?}");
            }
            let _ = spec.base_of(&selection);
        }
        assert!(spec.patches_of(PatchScope::Version(0)).is_empty());
        assert!(spec.patches_of(PatchScope::Version(usize::MAX)).is_empty());
        assert_eq!(
            spec.last_patch_up_to(usize::MAX).unwrap().content_uri,
            "v1b"
        );
    }

    #[test]
    fn maps_a_draft() {
        let spec = draft();

        assert!(spec.is_draft());
        assert_eq!(spec.status.visibility(), Some(Visibility::Public));
        assert_eq!(spec.versions[1].number, 2);
        assert_eq!(spec.unreleased_patches[0].content_uri, "u");
        assert_eq!(spec.proposals[0].status, ProposalState::ReadyForReview);
    }

    #[test]
    fn maps_a_published_spec() {
        let spec = published(vec![version(&["v1"])]);

        assert!(!spec.is_draft());
        assert_eq!(spec.status, SpecStatus::Published);
        assert!(spec.unreleased_patches.is_empty());
        assert!(spec.proposals.is_empty());
    }

    #[test]
    fn contains_only_existing_selections() {
        let spec = draft();

        assert!(spec.contains(&Selection::Version(1)));
        assert!(spec.contains(&Selection::Version(2)));
        assert!(!spec.contains(&Selection::Version(0)));
        assert!(!spec.contains(&Selection::Version(3)));
        assert!(spec.contains(&Selection::Unreleased));
        assert!(spec.contains(&Selection::Proposal("p".into())));
        assert!(!spec.contains(&Selection::Proposal("missing".into())));
        assert!(!published(vec![]).contains(&Selection::Unreleased));
    }

    #[test]
    fn selects_the_latest_version_by_default() {
        assert_eq!(draft().default_selection(), Some(Selection::Version(2)));
        assert_eq!(published(vec![]).default_selection(), None);

        let without_versions = SpecDetail {
            versions: Vec::new(),
            ..draft()
        };
        assert_eq!(
            without_versions.default_selection(),
            Some(Selection::Unreleased)
        );
    }

    #[test]
    fn the_text_is_the_one_of_the_last_patch() {
        let spec = draft();

        assert_eq!(spec.content_uri(&Selection::Version(1)), Some("v1b"));
        // Version 2 has no patches, so it has the text of version 1.
        assert_eq!(spec.content_uri(&Selection::Version(2)), Some("v1b"));
        assert_eq!(spec.content_uri(&Selection::Unreleased), Some("u"));
        assert_eq!(
            spec.content_uri(&Selection::Proposal("p".into())),
            Some("proposal")
        );
        assert_eq!(
            published(vec![version(&[])]).content_uri(&Selection::Version(1)),
            None
        );
    }

    #[test]
    fn diffs_against_the_text_before() {
        let spec = draft();

        assert_eq!(spec.base_of(&Selection::Version(1)), None);
        assert_eq!(
            spec.base_of(&Selection::Version(2)),
            Some(Selection::Version(1))
        );
        assert_eq!(
            spec.base_of(&Selection::Unreleased),
            Some(Selection::Version(2))
        );
        assert_eq!(
            spec.base_of(&Selection::Proposal("p".into())),
            Some(Selection::Unreleased)
        );

        let without_unreleased = SpecDetail {
            unreleased_patches: Vec::new(),
            ..draft()
        };
        assert_eq!(
            without_unreleased.base_of(&Selection::Proposal("p".into())),
            Some(Selection::Version(2))
        );
    }

    /// Knows a published spec `spec` with version 1 with the patches `p1` and `p2`, a provider
    /// that is `down`, and no other spec.
    fn spec_gateway() -> MockSpecGateway {
        let mut gateway = MockSpecGateway::new();
        gateway.expect_get_spec().returning(|id| match id {
            "spec" => Ok(Kind::Published(Spec {
                id: id.into(),
                versions: vec![version(&["p1", "p2"])],
                ..Default::default()
            })),
            "down" => Err(SpecGatewayError::Status(tonic::Status::unavailable("down"))),
            _ => Err(SpecGatewayError::Status(tonic::Status::not_found(
                "no such spec",
            ))),
        });
        gateway
    }

    /// Has the texts `p1` and `p2`, and answers `404` for any other.
    fn content_gateway() -> MockContentGateway {
        let mut gateway = MockContentGateway::new();
        gateway.expect_get().returning(|uri| match uri {
            "p1" => Ok("# One\n".to_string()),
            "p2" => Ok("# Two\n".to_string()),
            _ => Err(GetContentError::Status(404, uri.to_string())),
        });
        gateway
    }

    fn service(
        spec_gateway: MockSpecGateway,
        content_gateway: MockContentGateway,
    ) -> SpecViewService {
        SpecViewService::new(Arc::new(spec_gateway), Arc::new(content_gateway))
    }

    async fn view_of(
        id: &str,
        selection: Option<Selection>,
        mode: ViewMode,
    ) -> Result<SpecView, ViewSpecError> {
        service(spec_gateway(), content_gateway())
            .view_spec(id, selection, mode)
            .await
    }

    #[tokio::test]
    async fn shows_the_latest_version_rendered_by_default() {
        let view = view_of("spec", None, ViewMode::Rendered).await.unwrap();

        assert_eq!(view.selection, Some(Selection::Version(1)));
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn loads_the_diff_and_the_text_of_a_patch() {
        let view = view_of("spec", Some(Selection::Version(1)), ViewMode::Diff)
            .await
            .unwrap();
        assert!(matches!(&view.body, Body::Diff { base: None, diff } if diff.added == 1));

        let view = view_of(
            "spec",
            Some(Selection::Patch {
                scope: PatchScope::Version(1),
                number: 2,
            }),
            ViewMode::Rendered,
        )
        .await
        .unwrap();
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn maps_unknown_specs_and_selections_to_not_found() {
        assert!(matches!(
            view_of("missing", None, ViewMode::Rendered).await,
            Err(ViewSpecError::NotFound)
        ));
        assert!(matches!(
            view_of("spec", Some(Selection::Version(2)), ViewMode::Rendered).await,
            Err(ViewSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn does_not_load_any_text_for_an_unknown_selection() {
        let mut content_gateway = MockContentGateway::new();
        content_gateway.expect_get().never();

        let result = service(spec_gateway(), content_gateway)
            .view_spec("spec", Some(Selection::Version(2)), ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::NotFound)));
    }

    #[tokio::test]
    async fn maps_a_malformed_id_to_not_found() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway.expect_get_spec().returning(|_| {
            Err(SpecGatewayError::Status(tonic::Status::invalid_argument(
                "The provided spec ID is invalid.",
            )))
        });

        let result = service(spec_gateway, MockContentGateway::new())
            .view_spec("not-a-uuid", None, ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::NotFound)));
    }

    #[tokio::test]
    async fn maps_an_empty_provider_response_to_a_provider_error() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway
            .expect_get_spec()
            .returning(|_| Err(SpecGatewayError::MissingKind));

        let result = service(spec_gateway, MockContentGateway::new())
            .view_spec("spec", None, ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::Provider(_))));
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_a_provider_error() {
        assert!(matches!(
            view_of("down", None, ViewMode::Rendered).await,
            Err(ViewSpecError::Provider(_))
        ));
    }

    #[tokio::test]
    async fn still_shows_the_spec_when_its_text_is_unavailable() {
        let mut content_gateway = MockContentGateway::new();
        content_gateway
            .expect_get()
            .returning(|uri| Err(GetContentError::Request(format!("{uri} is unreachable"))));

        let view = service(spec_gateway(), content_gateway)
            .view_spec("spec", None, ViewMode::Rendered)
            .await
            .unwrap();

        assert!(matches!(view.body, Body::Unavailable));
    }
}
