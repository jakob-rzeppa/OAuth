use serde::{Deserialize, Deserializer};

use crate::application::spec_view::{PatchScope, Selection, ViewMode};

/// A url query that doesn't address anything.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct InvalidParams;

/// The url query of the detail page.
///
/// `version` is a version number or `unreleased`, `proposal` a proposal id and wins over
/// `version`, `patch` is the 1-based number of a patch of that version or of the unreleased
/// changes, and `view=diff` shows the changes instead of the text. An unknown `view` shows the
/// text.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct ViewParams {
    #[serde(deserialize_with = "version_param")]
    version: VersionParam,
    #[serde(deserialize_with = "patch_param")]
    patch: PatchParam,
    proposal: String,
    #[serde(deserialize_with = "lenient_view_mode")]
    view: ViewMode,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum VersionParam {
    #[default]
    Absent,
    Unreleased,
    Number(usize),
    /// Neither a number nor `unreleased`.
    Invalid,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum PatchParam {
    #[default]
    Absent,
    Number(usize),
    Invalid,
}

fn version_param<'de, D: Deserializer<'de>>(deserializer: D) -> Result<VersionParam, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "" => VersionParam::Absent,
        "unreleased" => VersionParam::Unreleased,
        version => version
            .parse()
            .map_or(VersionParam::Invalid, VersionParam::Number),
    })
}

fn patch_param<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PatchParam, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "" => PatchParam::Absent,
        patch => patch
            .parse()
            .map_or(PatchParam::Invalid, PatchParam::Number),
    })
}

fn lenient_view_mode<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ViewMode, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "diff" => ViewMode::Diff,
        _ => ViewMode::Rendered,
    })
}

impl ViewParams {
    /// What the url addresses, `None` for the default selection of the spec.
    pub fn parse(self) -> Result<(Option<Selection>, ViewMode), InvalidParams> {
        let scope = match self.version {
            VersionParam::Absent => None,
            VersionParam::Unreleased => Some(PatchScope::Unreleased),
            VersionParam::Number(number) => Some(PatchScope::Version(number)),
            VersionParam::Invalid if !self.proposal.is_empty() => None,
            VersionParam::Invalid => return Err(InvalidParams),
        };
        let selection = match (self.proposal, scope, self.patch) {
            (proposal, _, _) if !proposal.is_empty() => Some(Selection::Proposal(proposal)),
            (_, None, PatchParam::Absent) => None,
            (_, Some(PatchScope::Version(number)), PatchParam::Absent) => {
                Some(Selection::Version(number))
            }
            (_, Some(PatchScope::Unreleased), PatchParam::Absent) => Some(Selection::Unreleased),
            (_, Some(scope), PatchParam::Number(number)) => {
                Some(Selection::Patch { scope, number })
            }
            // A patch is only addressable within its version.
            (_, None, PatchParam::Number(_)) => return Err(InvalidParams),
            (_, _, PatchParam::Invalid) => return Err(InvalidParams),
        };
        Ok((selection, self.view))
    }
}

/// The url query that addresses the selection, the inverse of [`ViewParams::parse`].
pub(super) fn query_string(selection: Option<&Selection>, mode: ViewMode) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(query: &str) -> Result<(Option<Selection>, ViewMode), InvalidParams> {
        serde_urlencoded::from_str::<ViewParams>(query)
            .unwrap()
            .parse()
    }

    #[test]
    fn parses_the_url_query() {
        assert_eq!(parse(""), Ok((None, ViewMode::Rendered)));
        assert_eq!(
            parse("version=2&view=diff"),
            Ok((Some(Selection::Version(2)), ViewMode::Diff))
        );
        assert_eq!(
            parse("version=unreleased&view=other"),
            Ok((Some(Selection::Unreleased), ViewMode::Rendered))
        );
        assert_eq!(
            parse("version=&patch=&proposal=&view="),
            Ok((None, ViewMode::Rendered))
        );
    }

    #[test]
    fn a_proposal_wins_over_the_version() {
        let proposal = Some(Selection::Proposal("p".into()));

        assert_eq!(
            parse("version=2&proposal=p"),
            Ok((proposal.clone(), ViewMode::Rendered))
        );
        assert_eq!(
            parse("version=latest&proposal=p"),
            Ok((proposal, ViewMode::Rendered))
        );
    }

    #[test]
    fn parses_a_patch() {
        assert_eq!(
            parse("version=1&patch=2"),
            Ok((
                Some(Selection::Patch {
                    scope: PatchScope::Version(1),
                    number: 2
                }),
                ViewMode::Rendered
            ))
        );
        assert_eq!(
            parse("version=unreleased&patch=1").unwrap().0,
            Some(Selection::Patch {
                scope: PatchScope::Unreleased,
                number: 1
            })
        );
    }

    #[test]
    fn rejects_urls_that_address_nothing() {
        for query in [
            "version=latest",
            "version=-1",
            "patch=1",
            "version=1&patch=first",
        ] {
            assert_eq!(parse(query), Err(InvalidParams), "{query}");
        }
    }

    #[test]
    fn builds_the_url_query_of_a_selection() {
        let patch = Selection::Patch {
            scope: PatchScope::Version(1),
            number: 2,
        };

        assert_eq!(query_string(None, ViewMode::Rendered), "");
        assert_eq!(
            query_string(Some(&patch), ViewMode::Diff),
            "version=1&patch=2&view=diff"
        );
        assert_eq!(
            query_string(Some(&Selection::Unreleased), ViewMode::Diff),
            "version=unreleased&view=diff"
        );
        assert_eq!(
            query_string(Some(&Selection::Proposal("a b".into())), ViewMode::Diff),
            "proposal=a+b&view=diff"
        );
    }

    #[test]
    fn the_built_query_parses_back_to_the_selection() {
        let selections = [
            Selection::Version(3),
            Selection::Unreleased,
            Selection::Proposal("a b/c".into()),
            Selection::Patch {
                scope: PatchScope::Unreleased,
                number: 2,
            },
        ];

        for selection in selections {
            for mode in [ViewMode::Rendered, ViewMode::Diff] {
                assert_eq!(
                    parse(&query_string(Some(&selection), mode)),
                    Ok((Some(selection.clone()), mode))
                );
            }
        }
    }
}
