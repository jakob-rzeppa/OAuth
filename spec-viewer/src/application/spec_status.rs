use spec_api::{DraftVisibility, ProposalStatus};

/// Whether a spec is published or still a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecStatus {
    Published,
    Draft { visibility: Visibility },
}

impl SpecStatus {
    pub fn is_draft(&self) -> bool {
        matches!(self, SpecStatus::Draft { .. })
    }

    /// `None` for a published spec.
    pub fn visibility(&self) -> Option<Visibility> {
        match self {
            SpecStatus::Published => None,
            SpecStatus::Draft { visibility } => Some(*visibility),
        }
    }
}

/// Who can see a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Hidden,
    /// The spec-provider sent a value this viewer doesn't know.
    Unknown,
}

impl From<DraftVisibility> for Visibility {
    fn from(visibility: DraftVisibility) -> Self {
        match visibility {
            DraftVisibility::Public => Visibility::Public,
            DraftVisibility::Private => Visibility::Private,
            DraftVisibility::Hidden => Visibility::Hidden,
            DraftVisibility::Unspecified => Visibility::Unknown,
        }
    }
}

/// How far a proposal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalState {
    WorkInProgress,
    ReadyForReview,
    /// The spec-provider sent a value this viewer doesn't know.
    Unknown,
}

impl From<ProposalStatus> for ProposalState {
    fn from(status: ProposalStatus) -> Self {
        match status {
            ProposalStatus::Draft => ProposalState::WorkInProgress,
            ProposalStatus::Ready => ProposalState::ReadyForReview,
            ProposalStatus::Unspecified => ProposalState::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_has_a_visibility_and_a_published_spec_has_none() {
        let draft = SpecStatus::Draft {
            visibility: DraftVisibility::Private.into(),
        };

        assert!(draft.is_draft());
        assert_eq!(draft.visibility(), Some(Visibility::Private));
        assert!(!SpecStatus::Published.is_draft());
        assert_eq!(SpecStatus::Published.visibility(), None);
    }

    #[test]
    fn maps_unspecified_values_to_unknown() {
        assert_eq!(
            Visibility::from(DraftVisibility::Unspecified),
            Visibility::Unknown
        );
        assert_eq!(
            ProposalState::from(ProposalStatus::Unspecified),
            ProposalState::Unknown
        );
        assert_eq!(
            ProposalState::from(ProposalStatus::Ready),
            ProposalState::ReadyForReview
        );
    }
}
