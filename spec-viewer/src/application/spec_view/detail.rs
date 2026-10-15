use chrono::{DateTime, Utc};
use spec_api::{Patch, Proposal, Version, get_spec_response::Kind};

use crate::application::{
    spec_status::{ProposalState, SpecStatus},
    spec_view::{PatchScope, Selection},
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
    pub(super) fn default_selection(&self) -> Option<Selection> {
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
    pub(super) fn base_of(&self, selection: &Selection) -> Option<Selection> {
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
    pub(super) fn content_uri(&self, selection: &Selection) -> Option<&str> {
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
}
