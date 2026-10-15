//! How the data of the application is shown on the pages.

use chrono::{DateTime, Utc};

use crate::application::{
    diff::LineKind,
    spec_status::{ProposalState, SpecStatus, Visibility},
};

/// Shown for a missing or out of range time.
const UNKNOWN_TIME: &str = "unknown";

/// The text of a status, visibility or state.
pub trait Label {
    fn label(&self) -> &'static str;
}

impl Label for SpecStatus {
    fn label(&self) -> &'static str {
        match self {
            SpecStatus::Published => "Published",
            SpecStatus::Draft { .. } => "Draft",
        }
    }
}

impl Label for Visibility {
    fn label(&self) -> &'static str {
        match self {
            Visibility::Public => "Public",
            Visibility::Private => "Private",
            Visibility::Hidden => "Hidden",
            Visibility::Unknown => "Unknown",
        }
    }
}

impl Label for ProposalState {
    fn label(&self) -> &'static str {
        match self {
            ProposalState::WorkInProgress => "Work in progress",
            ProposalState::ReadyForReview => "Ready for review",
            ProposalState::Unknown => "Unknown",
        }
    }
}

/// The class the stylesheet colors a status or a diff line with.
pub trait CssClass {
    fn css_class(&self) -> &'static str;
}

impl CssClass for SpecStatus {
    fn css_class(&self) -> &'static str {
        match self {
            SpecStatus::Published => "published",
            SpecStatus::Draft { .. } => "draft",
        }
    }
}

impl CssClass for LineKind {
    fn css_class(&self) -> &'static str {
        match self {
            LineKind::Unchanged => "unchanged",
            LineKind::Added => "added",
            LineKind::Removed => "removed",
        }
    }
}

/// The sign shown in front of a diff line, like in a unified diff.
pub trait Sign {
    fn sign(&self) -> &'static str;
}

impl Sign for LineKind {
    fn sign(&self) -> &'static str {
        match self {
            LineKind::Unchanged => " ",
            LineKind::Added => "+",
            LineKind::Removed => "-",
        }
    }
}

/// The text of the version a draft is at.
pub trait VersionLabel {
    /// Like `Version 3`, or `No version yet`.
    fn version_label(&self) -> String;
}

impl VersionLabel for u32 {
    fn version_label(&self) -> String {
        match self {
            0 => "No version yet".to_string(),
            version => format!("Version {version}"),
        }
    }
}

/// A time shown in UTC.
pub trait TimeDisplay {
    /// Like `2024-01-11`.
    fn date(&self) -> String;

    /// Like `2024-01-11 18:13 UTC`.
    fn date_time(&self) -> String;
}

impl TimeDisplay for Option<DateTime<Utc>> {
    fn date(&self) -> String {
        format(self, "%Y-%m-%d")
    }

    fn date_time(&self) -> String {
        format(self, "%Y-%m-%d %H:%M UTC")
    }
}

fn format(time: &Option<DateTime<Utc>>, pattern: &str) -> String {
    time.map_or_else(
        || UNKNOWN_TIME.to_string(),
        |time| time.format(pattern).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use spec_api::DraftVisibility;

    #[test]
    fn labels_the_status_visibility_and_state() {
        let draft = SpecStatus::Draft {
            visibility: Visibility::Private,
        };

        assert_eq!(SpecStatus::Published.label(), "Published");
        assert_eq!(draft.label(), "Draft");
        assert_eq!(Visibility::from(DraftVisibility::Hidden).label(), "Hidden");
        assert_eq!(Visibility::Unknown.label(), "Unknown");
        assert_eq!(ProposalState::WorkInProgress.label(), "Work in progress");
        assert_eq!(ProposalState::ReadyForReview.label(), "Ready for review");
    }

    #[test]
    fn classes_the_status_and_the_diff_lines() {
        assert_eq!(SpecStatus::Published.css_class(), "published");
        assert_eq!(LineKind::Added.css_class(), "added");
        assert_eq!(LineKind::Removed.css_class(), "removed");
        assert_eq!(LineKind::Added.sign(), "+");
        assert_eq!(LineKind::Unchanged.sign(), " ");
    }

    #[test]
    fn labels_the_version_of_a_draft() {
        assert_eq!(0.version_label(), "No version yet");
        assert_eq!(2.version_label(), "Version 2");
    }

    #[test]
    fn formats_times_in_utc() {
        let time = DateTime::from_timestamp(1_705_000_000, 0);

        assert_eq!(time.date(), "2024-01-11");
        assert_eq!(time.date_time(), "2024-01-11 19:06 UTC");
    }

    #[test]
    fn formats_a_missing_time_as_unknown() {
        assert_eq!(None.date(), UNKNOWN_TIME);
        assert_eq!(None.date_time(), UNKNOWN_TIME);
    }
}
