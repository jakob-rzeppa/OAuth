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
