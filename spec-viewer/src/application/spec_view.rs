use std::sync::Arc;

use serde::Deserialize;
use spec_api::{
    DraftVisibility, Patch, Proposal, ProposalStatus, Version, get_spec_response::Kind,
};

use crate::application::{
    contract::{
        content_gateway::{ContentGateway, GetContentError},
        spec_gateway::{GetSpecError, SpecGateway},
    },
    diff::{Diff, diff},
    markdown::render_markdown,
    timestamp::format_date_time,
};

/// A published spec or a draft with everything the detail page shows besides its text.
#[derive(Debug)]
pub struct SpecDetail {
    pub id: String,
    pub title: String,
    pub abbreviation: String,
    /// `Published` or `Draft`.
    pub status: &'static str,
    /// `None` for a published spec.
    pub visibility: Option<&'static str>,
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
    pub created_at: String,
    pub accepted_by: String,
    pub accepted_at: String,
    /// Where the full text of the spec with this patch applied is stored.
    pub content_uri: String,
}

#[derive(Debug)]
pub struct ProposalInfo {
    pub id: String,
    pub proposed_by: String,
    pub created_at: String,
    pub last_updated_at: String,
    pub status: &'static str,
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
    fn scope(&self) -> Option<PatchScope> {
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

/// The url query of the detail page.
///
/// `version` is a version number or `unreleased`, `proposal` a proposal id and wins over
/// `version`, `patch` is the 1-based number of a patch of that version or of the unreleased
/// changes, and `view=diff` shows the changes instead of the text.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ViewParams {
    pub version: String,
    pub patch: String,
    pub proposal: String,
    pub view: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ViewSpecError {
    #[error("no such spec, version or proposal")]
    NotFound,
    #[error(transparent)]
    Provider(GetSpecError),
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
        base_label: Option<String>,
        diff: Diff,
    },
    /// The text could not be loaded from the content storage.
    Unavailable,
}

pub struct PatchLink<'a> {
    pub patch: &'a PatchInfo,
    pub query: String,
    pub selected: bool,
}

pub struct VersionOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
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

    /// Loads the spec with the given id and the text of the selection in `params`.
    ///
    /// Without a selection, the latest version is shown. When the text can not be loaded the
    /// view is still returned, with [`Body::Unavailable`].
    pub async fn view_spec(&self, id: &str, params: ViewParams) -> Result<SpecView, ViewSpecError> {
        let (selection, mode) = params.parse()?;

        let spec = match self.spec_gateway.get_spec(id).await {
            Ok(kind) => SpecDetail::from(kind),
            Err(GetSpecError::Status(status)) if status.code() == tonic::Code::NotFound => {
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
                        base_label: base.map(|base| spec.label(&base)),
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

impl ViewParams {
    fn parse(self) -> Result<(Option<Selection>, ViewMode), ViewSpecError> {
        let scope = match self.version.as_str() {
            "" => None,
            "unreleased" => Some(PatchScope::Unreleased),
            version => Some(PatchScope::Version(
                version.parse().map_err(|_| ViewSpecError::NotFound)?,
            )),
        };
        let selection = match (self.proposal.as_str(), scope, self.patch.as_str()) {
            (proposal, _, _) if !proposal.is_empty() => {
                Some(Selection::Proposal(proposal.to_string()))
            }
            (_, None, "") => None,
            (_, Some(PatchScope::Version(number)), "") => Some(Selection::Version(number)),
            (_, Some(PatchScope::Unreleased), "") => Some(Selection::Unreleased),
            (_, Some(scope), number) => Some(Selection::Patch {
                scope,
                number: number.parse().map_err(|_| ViewSpecError::NotFound)?,
            }),
            // A patch is only addressable within its version.
            (_, None, _) => return Err(ViewSpecError::NotFound),
        };
        let mode = match self.view.as_str() {
            "diff" => ViewMode::Diff,
            _ => ViewMode::Rendered,
        };
        Ok((selection, mode))
    }
}

impl SpecDetail {
    pub fn is_draft(&self) -> bool {
        self.visibility.is_some()
    }

    fn contains(&self, selection: &Selection) -> bool {
        match selection {
            Selection::Version(number) => (1..=self.versions.len()).contains(number),
            Selection::Unreleased => !self.unreleased_patches.is_empty(),
            Selection::Proposal(id) => self.proposal(id).is_some(),
            Selection::Patch { scope, number } => {
                self.has_scope(*scope) && (1..=self.patches_of(*scope).len()).contains(number)
            }
        }
    }

    fn has_scope(&self, scope: PatchScope) -> bool {
        match scope {
            PatchScope::Version(number) => (1..=self.versions.len()).contains(&number),
            PatchScope::Unreleased => !self.unreleased_patches.is_empty(),
        }
    }

    /// The patches of a scope that [`Self::has_scope`].
    fn patches_of(&self, scope: PatchScope) -> &[PatchInfo] {
        match scope {
            PatchScope::Version(number) => &self.versions[number - 1].patches,
            PatchScope::Unreleased => &self.unreleased_patches,
        }
    }

    /// The last patch of the newest version up to `count` that has any.
    fn last_patch_up_to(&self, count: usize) -> Option<&PatchInfo> {
        self.versions[..count]
            .iter()
            .rev()
            .find_map(|version| version.patches.last())
    }

    fn proposal(&self, id: &str) -> Option<&ProposalInfo> {
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
            Selection::Version(number) => {
                Some(number - 1).filter(|&n| n > 0).map(Selection::Version)
            }
            Selection::Unreleased => self.latest_version(),
            Selection::Proposal(_) => self.head(),
            Selection::Patch { scope, number } if *number > 1 => Some(Selection::Patch {
                scope: *scope,
                number: number - 1,
            }),
            // The first patch changes what the patches before it left.
            Selection::Patch { scope, .. } => {
                let before = match scope {
                    PatchScope::Version(number) => number - 1,
                    PatchScope::Unreleased => self.versions.len(),
                };
                self.versions[..before]
                    .iter()
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
            Selection::Version(number) => self.last_patch_up_to(*number),
            Selection::Unreleased => self
                .unreleased_patches
                .last()
                .or_else(|| self.last_patch_up_to(self.versions.len())),
            Selection::Patch { scope, number } => self.patches_of(*scope).get(number - 1),
            Selection::Proposal(id) => {
                return self
                    .proposal(id)
                    .map(|proposal| proposal.content_uri.as_str());
            }
        }
        .map(|patch| patch.content_uri.as_str())
    }

    fn label(&self, selection: &Selection) -> String {
        match selection {
            Selection::Version(number) => format!("Version {number}"),
            Selection::Unreleased => "Unreleased changes".to_string(),
            Selection::Proposal(id) => match self.proposal(id) {
                Some(proposal) => format!("Proposal by {}", proposal.proposed_by),
                None => "Proposal".to_string(),
            },
            Selection::Patch { scope, number } => {
                let scope = match scope {
                    PatchScope::Version(number) => format!("Version {number}"),
                    PatchScope::Unreleased => "Unreleased changes".to_string(),
                };
                format!("{scope}, patch {number}")
            }
        }
    }
}

impl SpecView {
    pub fn is_diff(&self) -> bool {
        self.mode == ViewMode::Diff
    }

    /// Like `Version 2`, `None` when the spec has no text yet.
    pub fn selection_label(&self) -> Option<String> {
        self.selection
            .as_ref()
            .map(|selection| self.spec.label(selection))
    }

    pub fn version_options(&self) -> Vec<VersionOption> {
        let scope = self.selection.as_ref().and_then(Selection::scope);
        let unreleased = self
            .spec
            .contains(&Selection::Unreleased)
            .then(|| VersionOption {
                value: "unreleased".to_string(),
                label: self.spec.label(&Selection::Unreleased),
                selected: scope == Some(PatchScope::Unreleased),
            });
        let latest = self.spec.versions.len();
        let versions = self
            .spec
            .versions
            .iter()
            .rev()
            .map(|version| VersionOption {
                value: version.number.to_string(),
                label: if version.number == latest {
                    format!("Version {} (latest)", version.number)
                } else {
                    format!("Version {}", version.number)
                },
                selected: scope == Some(PatchScope::Version(version.number)),
            });
        unreleased.into_iter().chain(versions).collect()
    }

    /// Whether the selection is a single patch, rather than the whole version.
    pub fn is_patch_selected(&self) -> bool {
        matches!(self.selection, Some(Selection::Patch { .. }))
    }

    /// The patches of the selected version or of the unreleased changes, each with the url
    /// query that shows it, keeping the view mode.
    pub fn patch_links(&self) -> Vec<PatchLink<'_>> {
        let Some(scope) = self.selection.as_ref().and_then(Selection::scope) else {
            return Vec::new();
        };
        self.spec
            .patches_of(scope)
            .iter()
            .enumerate()
            .map(|(index, patch)| {
                let selection = Selection::Patch {
                    scope,
                    number: index + 1,
                };
                PatchLink {
                    patch,
                    query: query_string(Some(&selection), self.mode),
                    selected: self.selection.as_ref() == Some(&selection),
                }
            })
            .collect()
    }

    /// The url query of the whole version or the unreleased changes, keeping the view mode.
    pub fn scope_query_string(&self) -> String {
        let selection = match self.selection.as_ref().and_then(Selection::scope) {
            Some(PatchScope::Version(number)) => Some(Selection::Version(number)),
            Some(PatchScope::Unreleased) => Some(Selection::Unreleased),
            None => None,
        };
        query_string(selection.as_ref(), self.mode)
    }

    pub fn selected_proposal(&self) -> Option<&ProposalInfo> {
        match &self.selection {
            Some(Selection::Proposal(id)) => self.spec.proposal(id),
            _ => None,
        }
    }

    pub fn is_selected_proposal(&self, id: &str) -> bool {
        matches!(&self.selection, Some(Selection::Proposal(selected)) if selected == id)
    }

    /// The url query of what is shown.
    pub fn query_string(&self) -> String {
        query_string(self.selection.as_ref(), self.mode)
    }

    /// The url query of the selection as rendered text.
    pub fn rendered_query_string(&self) -> String {
        query_string(self.selection.as_ref(), ViewMode::Rendered)
    }

    /// The url query of the selection as diff.
    pub fn diff_query_string(&self) -> String {
        query_string(self.selection.as_ref(), ViewMode::Diff)
    }

    /// The url query of the proposal, keeping the view mode.
    pub fn proposal_query_string(&self, id: &str) -> String {
        query_string(Some(&Selection::Proposal(id.to_string())), self.mode)
    }
}

fn query_string(selection: Option<&Selection>, mode: ViewMode) -> String {
    let mut pairs = Vec::new();
    match selection {
        Some(Selection::Version(number)) => pairs.push(("version", number.to_string())),
        Some(Selection::Unreleased) => pairs.push(("version", "unreleased".to_string())),
        Some(Selection::Proposal(id)) => pairs.push(("proposal", id.clone())),
        Some(Selection::Patch { scope, number }) => {
            pairs.push((
                "version",
                match scope {
                    PatchScope::Version(number) => number.to_string(),
                    PatchScope::Unreleased => "unreleased".to_string(),
                },
            ));
            pairs.push(("patch", number.to_string()));
        }
        None => {}
    }
    if mode == ViewMode::Diff {
        pairs.push(("view", "diff".to_string()));
    }
    serde_urlencoded::to_string(pairs).expect("string pairs always encode")
}

pub fn visibility_label(visibility: DraftVisibility) -> &'static str {
    match visibility {
        DraftVisibility::Public => "Public",
        DraftVisibility::Private => "Private",
        DraftVisibility::Hidden => "Hidden",
        DraftVisibility::Unspecified => "Unknown",
    }
}

impl From<Kind> for SpecDetail {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Published(spec) => SpecDetail {
                id: spec.id,
                title: spec.title,
                abbreviation: spec.abbreviation,
                status: "Published",
                visibility: None,
                versions: versions(spec.versions),
                unreleased_patches: Vec::new(),
                proposals: Vec::new(),
            },
            Kind::Draft(draft) => SpecDetail {
                status: "Draft",
                visibility: Some(visibility_label(draft.visibility())),
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
            created_at: format_date_time(patch.created_at.as_ref()),
            accepted_at: format_date_time(patch.accepted_at.as_ref()),
            proposed_by: patch.proposed_by,
            accepted_by: patch.accepted_by,
            content_uri: patch.content_uri,
        }
    }
}

impl From<Proposal> for ProposalInfo {
    fn from(proposal: Proposal) -> Self {
        ProposalInfo {
            created_at: format_date_time(proposal.created_at.as_ref()),
            last_updated_at: format_date_time(proposal.last_updated_at.as_ref()),
            status: match proposal.status() {
                ProposalStatus::Draft => "Work in progress",
                ProposalStatus::Ready => "Ready for review",
                ProposalStatus::Unspecified => "Unknown",
            },
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
    use spec_api::{Draft, Spec};

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

    fn params(version: &str, proposal: &str, view: &str) -> ViewParams {
        ViewParams {
            version: version.into(),
            proposal: proposal.into(),
            view: view.into(),
            ..Default::default()
        }
    }

    fn patch_params(version: &str, patch: &str) -> ViewParams {
        ViewParams {
            version: version.into(),
            patch: patch.into(),
            ..Default::default()
        }
    }

    fn view(spec: SpecDetail, selection: Option<Selection>, mode: ViewMode) -> SpecView {
        SpecView {
            spec,
            selection,
            mode,
            body: Body::Empty,
        }
    }

    #[test]
    fn parses_the_url_query() {
        assert_eq!(
            params("", "", "").parse().unwrap(),
            (None, ViewMode::Rendered)
        );
        assert_eq!(
            params("2", "", "diff").parse().unwrap(),
            (Some(Selection::Version(2)), ViewMode::Diff)
        );
        assert_eq!(
            params("unreleased", "", "other").parse().unwrap(),
            (Some(Selection::Unreleased), ViewMode::Rendered)
        );
        assert_eq!(
            params("2", "p", "").parse().unwrap().0,
            Some(Selection::Proposal("p".into()))
        );
    }

    #[test]
    fn parses_a_patch() {
        assert_eq!(
            patch_params("1", "2").parse().unwrap().0,
            Some(Selection::Patch {
                scope: PatchScope::Version(1),
                number: 2
            })
        );
        assert_eq!(
            patch_params("unreleased", "1").parse().unwrap().0,
            Some(Selection::Patch {
                scope: PatchScope::Unreleased,
                number: 1
            })
        );
        assert!(matches!(
            patch_params("", "1").parse(),
            Err(ViewSpecError::NotFound)
        ));
        assert!(matches!(
            patch_params("1", "first").parse(),
            Err(ViewSpecError::NotFound)
        ));
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
        assert_eq!(
            spec.label(&patch(PatchScope::Unreleased, 1)),
            "Unreleased changes, patch 1"
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
    fn builds_the_url_queries_of_patches() {
        let selected = Selection::Patch {
            scope: PatchScope::Version(1),
            number: 2,
        };
        let view = view(draft(), Some(selected), ViewMode::Diff);

        assert_eq!(view.query_string(), "version=1&patch=2&view=diff");
        assert_eq!(view.scope_query_string(), "version=1&view=diff");
        assert!(view.is_patch_selected());
        let links = view.patch_links();
        assert_eq!(
            links
                .iter()
                .map(|link| (link.query.as_str(), link.selected))
                .collect::<Vec<_>>(),
            [
                ("version=1&patch=1&view=diff", false),
                ("version=1&patch=2&view=diff", true),
            ]
        );
        assert!(
            view.version_options()
                .iter()
                .any(|option| option.value == "1" && option.selected)
        );
    }

    #[test]
    fn rejects_an_invalid_version() {
        assert!(matches!(
            params("latest", "", "").parse(),
            Err(ViewSpecError::NotFound)
        ));
    }

    #[test]
    fn maps_a_draft() {
        let spec = draft();

        assert!(spec.is_draft());
        assert_eq!(spec.visibility, Some("Public"));
        assert_eq!(spec.versions[1].number, 2);
        assert_eq!(spec.unreleased_patches[0].content_uri, "u");
        assert_eq!(spec.proposals[0].status, "Ready for review");
    }

    #[test]
    fn maps_a_published_spec() {
        let spec = published(vec![version(&["v1"])]);

        assert!(!spec.is_draft());
        assert_eq!(spec.status, "Published");
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

    #[test]
    fn lists_the_unreleased_changes_and_the_newest_version_first() {
        let view = view(draft(), Some(Selection::Version(1)), ViewMode::Rendered);

        let options: Vec<_> = view
            .version_options()
            .into_iter()
            .map(|option| (option.value, option.label, option.selected))
            .collect();

        assert_eq!(
            options,
            [
                ("unreleased".into(), "Unreleased changes".into(), false),
                ("2".into(), "Version 2 (latest)".into(), false),
                ("1".into(), "Version 1".into(), true),
            ]
        );
    }

    #[test]
    fn builds_the_url_queries() {
        let view = view(draft(), Some(Selection::Unreleased), ViewMode::Diff);

        assert_eq!(view.query_string(), "version=unreleased&view=diff");
        assert_eq!(view.rendered_query_string(), "version=unreleased");
        assert_eq!(view.proposal_query_string("a b"), "proposal=a+b&view=diff");
    }

    #[test]
    fn labels_the_selection() {
        let view = view(
            draft(),
            Some(Selection::Proposal("p".into())),
            ViewMode::Rendered,
        );

        assert_eq!(view.selection_label().as_deref(), Some("Proposal by alice"));
        assert!(view.is_selected_proposal("p"));
        assert_eq!(view.selected_proposal().unwrap().proposed_by, "alice");
        assert!(view.patch_links().is_empty());
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
            "down" => Err(GetSpecError::Status(tonic::Status::unavailable("down"))),
            _ => Err(GetSpecError::Status(tonic::Status::not_found(
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

    async fn view_of(id: &str, params: ViewParams) -> Result<SpecView, ViewSpecError> {
        service(spec_gateway(), content_gateway())
            .view_spec(id, params)
            .await
    }

    #[tokio::test]
    async fn shows_the_latest_version_rendered_by_default() {
        let view = view_of("spec", ViewParams::default()).await.unwrap();

        assert_eq!(view.selection, Some(Selection::Version(1)));
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn loads_the_diff_and_the_text_of_a_patch() {
        let view = view_of("spec", params("1", "", "diff")).await.unwrap();
        assert!(matches!(&view.body, Body::Diff { base_label: None, diff } if diff.added == 1));

        let view = view_of("spec", patch_params("1", "2")).await.unwrap();
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn maps_unknown_specs_and_selections_to_not_found() {
        assert!(matches!(
            view_of("missing", ViewParams::default()).await,
            Err(ViewSpecError::NotFound)
        ));
        assert!(matches!(
            view_of("spec", params("2", "", "")).await,
            Err(ViewSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn does_not_load_any_text_for_an_unknown_selection() {
        let mut content_gateway = MockContentGateway::new();
        content_gateway.expect_get().never();

        let result = service(spec_gateway(), content_gateway)
            .view_spec("spec", params("2", "", ""))
            .await;

        assert!(matches!(result, Err(ViewSpecError::NotFound)));
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_a_provider_error() {
        assert!(matches!(
            view_of("down", ViewParams::default()).await,
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
            .view_spec("spec", ViewParams::default())
            .await
            .unwrap();

        assert!(matches!(view.body, Body::Unavailable));
    }
}
