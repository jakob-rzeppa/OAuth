use std::time::Duration;

use similar::{ChangeTag, TextDiff, udiff::UnifiedHunkHeader};

/// The number of unchanged lines shown around each change, like `git diff`.
const CONTEXT_LINES: usize = 3;

/// After this, the diff stops looking for a shorter one and takes what it has: still a correct
/// diff, just with more changed lines than needed.
const DIFF_TIMEOUT: Duration = Duration::from_secs(2);

/// A line-based diff, grouped into hunks like a unified diff.
#[derive(Debug)]
pub struct Diff {
    pub hunks: Vec<Hunk>,
    pub added: usize,
    pub removed: usize,
}

#[derive(Debug)]
pub struct Hunk {
    /// Like `@@ -1,4 +1,5 @@`.
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    /// The 1-based line number in the old text, `None` for added lines.
    pub old_number: Option<usize>,
    /// The 1-based line number in the new text, `None` for removed lines.
    pub new_number: Option<usize>,
    /// The line without its line break.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Unchanged,
    Added,
    Removed,
}

impl LineKind {
    /// The sign shown in front of the line, like in a unified diff.
    pub fn sign(&self) -> &'static str {
        match self {
            LineKind::Unchanged => " ",
            LineKind::Added => "+",
            LineKind::Removed => "-",
        }
    }

    pub fn css_class(&self) -> &'static str {
        match self {
            LineKind::Unchanged => "unchanged",
            LineKind::Added => "added",
            LineKind::Removed => "removed",
        }
    }
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty()
    }
}

/// Diffs `old` and `new` line by line.
pub fn diff(old: &str, new: &str) -> Diff {
    let text_diff = TextDiff::configure()
        .timeout(DIFF_TIMEOUT)
        .diff_lines(old, new);
    let mut added = 0;
    let mut removed = 0;

    let hunks = text_diff
        .grouped_ops(CONTEXT_LINES)
        .iter()
        .map(|ops| Hunk {
            header: UnifiedHunkHeader::new(ops).to_string(),
            lines: ops
                .iter()
                .flat_map(|op| text_diff.iter_changes(op))
                .map(|change| {
                    let kind = match change.tag() {
                        ChangeTag::Equal => LineKind::Unchanged,
                        ChangeTag::Insert => LineKind::Added,
                        ChangeTag::Delete => LineKind::Removed,
                    };
                    match kind {
                        LineKind::Added => added += 1,
                        LineKind::Removed => removed += 1,
                        LineKind::Unchanged => {}
                    }
                    DiffLine {
                        kind,
                        old_number: change.old_index().map(|index| index + 1),
                        new_number: change.new_index().map(|index| index + 1),
                        text: change.value().trim_end_matches(['\n', '\r']).to_string(),
                    }
                })
                .collect(),
        })
        .collect();

    Diff {
        hunks,
        added,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: LineKind, old: Option<usize>, new: Option<usize>, text: &str) -> DiffLine {
        DiffLine {
            kind,
            old_number: old,
            new_number: new,
            text: text.into(),
        }
    }

    #[test]
    fn equal_texts_have_no_hunks() {
        let diff = diff("a\nb\n", "a\nb\n");

        assert!(diff.is_empty());
        assert_eq!((diff.added, diff.removed), (0, 0));
    }

    #[test]
    fn numbers_the_lines_of_both_sides() {
        let diff = diff("a\nb\nc\n", "a\nB\nc\nd\n");

        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].header, "@@ -1,3 +1,4 @@");
        assert_eq!(
            diff.hunks[0].lines,
            [
                line(LineKind::Unchanged, Some(1), Some(1), "a"),
                line(LineKind::Removed, Some(2), None, "b"),
                line(LineKind::Added, None, Some(2), "B"),
                line(LineKind::Unchanged, Some(3), Some(3), "c"),
                line(LineKind::Added, None, Some(4), "d"),
            ]
        );
        assert_eq!((diff.added, diff.removed), (2, 1));
    }

    #[test]
    fn splits_distant_changes_into_hunks_with_context() {
        let old: String = (1..=20).map(|i| format!("{i}\n")).collect();
        let new = old.replace("2\n", "two\n").replace("19\n", "nineteen\n");

        let diff = diff(&old, &new);

        assert_eq!(diff.hunks.len(), 2);
        let first = &diff.hunks[0].lines;
        assert_eq!(first.first().unwrap().text, "1");
        assert_eq!(first.last().unwrap().text, "5");
    }

    #[test]
    fn a_large_diff_finishes_with_a_correct_result() {
        let old: String = (0..20_000).map(|i| format!("old {i}\n")).collect();
        let new: String = (0..20_000).map(|i| format!("new {i}\n")).collect();

        let started = std::time::Instant::now();
        let diff = diff(&old, &new);

        assert!(started.elapsed() < DIFF_TIMEOUT * 3);
        assert_eq!((diff.added, diff.removed), (20_000, 20_000));
    }

    #[test]
    fn diffs_against_an_empty_text() {
        let diff = diff("", "a\nb\n");

        assert_eq!(diff.added, 2);
        assert!(
            diff.hunks[0]
                .lines
                .iter()
                .all(|line| line.kind == LineKind::Added)
        );
    }
}
