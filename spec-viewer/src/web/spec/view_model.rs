use crate::{
    application::spec_view::{
        Body, PatchInfo, PatchScope, ProposalInfo, Selection, SpecDetail, SpecView, ViewMode,
    },
    web::spec::{content_path, page_path, params::query_string},
};

/// A spec as the templates of the detail page show it: the text of the application with the
/// labels and the urls of what can be shown instead.
#[derive(Clone, Copy)]
pub(super) struct SpecViewModel<'a> {
    pub spec: &'a SpecDetail,
    pub body: &'a Body,
    selection: Option<&'a Selection>,
    mode: ViewMode,
}

/// Where to go to show something else: the page for a plain link, and the partial that htmx
/// loads instead.
pub(super) struct Link {
    pub page_url: String,
    pub content_url: String,
}

pub(super) struct PatchLink<'a> {
    pub patch: &'a PatchInfo,
    pub link: Link,
    pub selected: bool,
}

pub(super) struct VersionOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

impl<'a> SpecViewModel<'a> {
    pub fn new(view: &'a SpecView) -> Self {
        SpecViewModel {
            spec: &view.spec,
            body: &view.body,
            selection: view.selection.as_ref(),
            mode: view.mode,
        }
    }

    pub fn is_diff(&self) -> bool {
        self.mode == ViewMode::Diff
    }

    /// Like `Version 2`, `None` when the spec has no text yet.
    pub fn selection_label(&self) -> Option<String> {
        self.selection.map(|selection| self.label_of(selection))
    }

    /// Like `Version 2`, or `Proposal by alice`.
    pub fn label_of(&self, selection: &Selection) -> String {
        match selection {
            Selection::Version(number) => format!("Version {number}"),
            Selection::Unreleased => "Unreleased changes".to_string(),
            Selection::Proposal(id) => match self.spec.proposal(id) {
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

    /// The unreleased changes and the versions, the newest first.
    pub fn version_options(&self) -> Vec<VersionOption> {
        let scope = self.selection.and_then(Selection::scope);
        let unreleased = self
            .spec
            .contains(&Selection::Unreleased)
            .then(|| VersionOption {
                value: "unreleased".to_string(),
                label: self.label_of(&Selection::Unreleased),
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

    /// The patches of the selected version or of the unreleased changes, each with the link
    /// that shows it, keeping the view mode.
    pub fn patch_links(&self) -> Vec<PatchLink<'a>> {
        let Some(scope) = self.selection.and_then(Selection::scope) else {
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
                    link: self.link(Some(&selection), self.mode),
                    selected: self.selection == Some(&selection),
                }
            })
            .collect()
    }

    /// The link to the whole version or the unreleased changes, keeping the view mode.
    pub fn scope_link(&self) -> Link {
        let selection = match self.selection.and_then(Selection::scope) {
            Some(PatchScope::Version(number)) => Some(Selection::Version(number)),
            Some(PatchScope::Unreleased) => Some(Selection::Unreleased),
            None => None,
        };
        self.link(selection.as_ref(), self.mode)
    }

    pub fn selected_proposal(&self) -> Option<&'a ProposalInfo> {
        match self.selection {
            Some(Selection::Proposal(id)) => self.spec.proposal(id),
            _ => None,
        }
    }

    pub fn is_selected_proposal(&self, id: &str) -> bool {
        matches!(self.selection, Some(Selection::Proposal(selected)) if selected == id)
    }

    /// The path of the page of the spec.
    pub fn page_path(&self) -> String {
        page_path(&self.spec.id)
    }

    /// The path of the partial of the page of the spec.
    pub fn content_path(&self) -> String {
        content_path(&self.spec.id)
    }

    /// The url of the page as it shows what is shown.
    pub fn current_url(&self) -> String {
        self.link(self.selection, self.mode).page_url
    }

    /// The link to the selection as rendered text.
    pub fn rendered_link(&self) -> Link {
        self.link(self.selection, ViewMode::Rendered)
    }

    /// The link to the selection as diff.
    pub fn diff_link(&self) -> Link {
        self.link(self.selection, ViewMode::Diff)
    }

    /// The link to the proposal, keeping the view mode.
    pub fn proposal_link(&self, id: &str) -> Link {
        self.link(Some(&Selection::Proposal(id.to_string())), self.mode)
    }

    fn link(&self, selection: Option<&Selection>, mode: ViewMode) -> Link {
        let query = query_string(selection, mode);
        let with_query = |path: String| {
            if query.is_empty() {
                path
            } else {
                format!("{path}?{query}")
            }
        };
        Link {
            page_url: with_query(self.page_path()),
            content_url: with_query(self.content_path()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::timestamp::TimestampService;
    use spec_api::{Draft, DraftVisibility, Patch, Proposal, ProposalStatus, Version};

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

    /// Version 1 with `v1a` and `v1b`, an empty version 2, unreleased `u` and proposal `p`
    /// by alice.
    fn draft() -> SpecDetail {
        SpecDetail::from_kind(
            spec_api::get_spec_response::Kind::Draft(Draft {
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
            }),
            &TimestampService::new(),
        )
    }

    fn view(selection: Option<Selection>, mode: ViewMode) -> SpecView {
        SpecView {
            spec: draft(),
            selection,
            mode,
            body: Body::Empty,
        }
    }

    #[test]
    fn builds_the_url_queries_of_patches() {
        let selected = Selection::Patch {
            scope: PatchScope::Version(1),
            number: 2,
        };
        let view = view(Some(selected), ViewMode::Diff);
        let model = SpecViewModel::new(&view);

        assert_eq!(
            model.current_url(),
            "/specs/draft?version=1&patch=2&view=diff"
        );
        assert_eq!(
            model.scope_link().page_url,
            "/specs/draft?version=1&view=diff"
        );
        assert!(model.is_patch_selected());
        let links = model.patch_links();
        assert_eq!(
            links
                .iter()
                .map(|link| (link.link.page_url.as_str(), link.selected))
                .collect::<Vec<_>>(),
            [
                ("/specs/draft?version=1&patch=1&view=diff", false),
                ("/specs/draft?version=1&patch=2&view=diff", true),
            ]
        );
        assert_eq!(
            links[1].link.content_url,
            "/specs/draft/content?version=1&patch=2&view=diff"
        );
        assert!(
            model
                .version_options()
                .iter()
                .any(|option| option.value == "1" && option.selected)
        );
    }

    #[test]
    fn lists_the_unreleased_changes_and_the_newest_version_first() {
        let view = view(Some(Selection::Version(1)), ViewMode::Rendered);

        let options: Vec<_> = SpecViewModel::new(&view)
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
        let view = view(Some(Selection::Unreleased), ViewMode::Diff);
        let model = SpecViewModel::new(&view);

        assert_eq!(
            model.current_url(),
            "/specs/draft?version=unreleased&view=diff"
        );
        assert_eq!(
            model.rendered_link().page_url,
            "/specs/draft?version=unreleased"
        );
        assert_eq!(
            model.diff_link().content_url,
            "/specs/draft/content?version=unreleased&view=diff"
        );
        assert_eq!(
            model.proposal_link("a b").page_url,
            "/specs/draft?proposal=a+b&view=diff"
        );
        assert_eq!(model.page_path(), "/specs/draft");
        assert_eq!(model.content_path(), "/specs/draft/content");
    }

    #[test]
    fn labels_the_selection() {
        let view = view(Some(Selection::Proposal("p".into())), ViewMode::Rendered);
        let model = SpecViewModel::new(&view);

        assert_eq!(
            model.selection_label().as_deref(),
            Some("Proposal by alice")
        );
        assert!(model.is_selected_proposal("p"));
        assert_eq!(model.selected_proposal().unwrap().proposed_by, "alice");
        assert!(model.patch_links().is_empty());
        assert_eq!(
            model.label_of(&Selection::Patch {
                scope: PatchScope::Unreleased,
                number: 1
            }),
            "Unreleased changes, patch 1"
        );
        assert_eq!(
            model.label_of(&Selection::Proposal("gone".into())),
            "Proposal"
        );
    }

    #[test]
    fn links_without_a_query_when_nothing_is_selected() {
        let view = view(None, ViewMode::Rendered);
        let model = SpecViewModel::new(&view);

        assert_eq!(model.current_url(), "/specs/draft");
        assert_eq!(model.rendered_link().content_url, "/specs/draft/content");
    }

    #[test]
    fn has_no_label_without_a_selection() {
        let view = view(None, ViewMode::Rendered);

        assert_eq!(SpecViewModel::new(&view).selection_label(), None);
    }
}
