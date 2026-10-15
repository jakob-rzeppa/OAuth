/// Whether a spec with this title and abbreviation matches the search term.
///
/// The term is matched as a case-insensitive substring against the title or the abbreviation.
/// A blank term matches everything.
pub fn matches_search(title: &str, abbreviation: &str, term: &str) -> bool {
    let term = term.trim().to_lowercase();
    title.to_lowercase().contains(&term) || abbreviation.to_lowercase().contains(&term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_substring_of_the_title() {
        assert!(matches_search("Open Banking API", "OBA", "banking"));
    }

    #[test]
    fn matches_a_substring_of_the_abbreviation() {
        assert!(matches_search("Open Banking API", "OBA", "ob"));
    }

    #[test]
    fn ignores_case() {
        assert!(matches_search("Open Banking API", "OBA", "OPEN BANKING"));
        assert!(matches_search("open banking api", "oba", "Oba"));
    }

    #[test]
    fn ignores_surrounding_whitespace_in_the_term() {
        assert!(matches_search("Open Banking API", "OBA", "  banking \n"));
    }

    #[test]
    fn a_blank_term_matches_everything() {
        assert!(matches_search("Open Banking API", "OBA", ""));
        assert!(matches_search("Open Banking API", "OBA", "   "));
    }

    #[test]
    fn does_not_match_when_neither_field_contains_the_term() {
        assert!(!matches_search("Open Banking API", "OBA", "payments"));
    }

    #[test]
    fn does_not_match_across_the_title_and_abbreviation() {
        assert!(!matches_search("Open Banking API", "OBA", "api oba"));
    }
}
