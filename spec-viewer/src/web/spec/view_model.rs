use crate::{
    application::spec_view::{
        Body, PatchInfo, PatchScope, ProposalInfo, Selection, SpecDetail, SpecView, ViewMode,
    },
    web::spec::params::query_string,
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

pub(super) struct PatchLink<'a> {
    pub patch: &'a PatchInfo,
    pub query: String,
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

    /// The patches of the selected version or of the unreleased changes, each with the url
    /// query that shows it, keeping the view mode.
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
                    query: query_string(Some(&selection), self.mode),
                    selected: self.selection == Some(&selection),
                }
            })
            .collect()
    }

    /// The url query of the whole version or the unreleased changes, keeping the view mode.
    pub fn scope_query_string(&self) -> String {
        let selection = match self.selection.and_then(Selection::scope) {
            Some(PatchScope::Version(number)) => Some(Selection::Version(number)),
            Some(PatchScope::Unreleased) => Some(Selection::Unreleased),
            None => None,
        };
        query_string(selection.as_ref(), self.mode)
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

    /// The url query of what is shown.
    pub fn query_string(&self) -> String {
        query_string(self.selection, self.mode)
    }

    /// The url query of the selection as rendered text.
    pub fn rendered_query_string(&self) -> String {
        query_string(self.selection, ViewMode::Rendered)
    }

    /// The url query of the selection as diff.
    pub fn diff_query_string(&self) -> String {
        query_string(self.selection, ViewMode::Diff)
    }

    /// The url query of the proposal, keeping the view mode.
    pub fn proposal_query_string(&self, id: &str) -> String {
        query_string(Some(&Selection::Proposal(id.to_string())), self.mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        SpecDetail::from(spec_api::get_spec_response::Kind::Draft(Draft {
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

        assert_eq!(model.query_string(), "version=1&patch=2&view=diff");
        assert_eq!(model.scope_query_string(), "version=1&view=diff");
        assert!(model.is_patch_selected());
        let links = model.patch_links();
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

        assert_eq!(model.query_string(), "version=unreleased&view=diff");
        assert_eq!(model.rendered_query_string(), "version=unreleased");
        assert_eq!(model.diff_query_string(), "version=unreleased&view=diff");
        assert_eq!(model.proposal_query_string("a b"), "proposal=a+b&view=diff");
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
    fn has_no_label_without_a_selection() {
        let view = view(None, ViewMode::Rendered);

        assert_eq!(SpecViewModel::new(&view).selection_label(), None);
    }
}
