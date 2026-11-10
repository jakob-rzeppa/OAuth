use crate::util::token::hash_token;

/// Verify the code_verifier against the code_challenge.
///
/// # code_challenge_method
///
/// "S256": S256 hashing and Base64URL encoding as per the PKCE specification.
pub(super) fn verify_code_challenge(
    code_challenge_method: &str,
    code_verifier: &str,
    code_challenge: &str,
) -> bool {
    // Only support S256 method
    if code_challenge_method != "S256" {
        return false;
    }

    hash_token(code_verifier) == code_challenge
}

/// A code verifier is 43 to 128 characters from the unreserved set `[A-Za-z0-9-._~]`
/// (RFC 7636 §4.1).
pub(super) fn is_valid_code_verifier(code_verifier: &str) -> bool {
    (43..=128).contains(&code_verifier.len())
        && code_verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_rfc_7636_example_verifier() {
        assert!(is_valid_code_verifier(
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        ));
    }

    #[test]
    fn accepts_every_unreserved_character() {
        assert!(is_valid_code_verifier(&format!(
            "AZaz09-._~{}",
            "a".repeat(33)
        )));
    }

    #[test]
    fn accepts_the_minimum_and_maximum_length() {
        assert!(is_valid_code_verifier(&"a".repeat(43)));
        assert!(is_valid_code_verifier(&"a".repeat(128)));
    }

    #[test]
    fn rejects_a_verifier_that_is_too_short_or_too_long() {
        assert!(!is_valid_code_verifier(""));
        assert!(!is_valid_code_verifier(&"a".repeat(42)));
        assert!(!is_valid_code_verifier(&"a".repeat(129)));
    }

    #[test]
    fn rejects_characters_outside_the_unreserved_set() {
        for character in ["+", "/", "=", " ", "%", "ä"] {
            let verifier = format!("{}{character}", "a".repeat(42));
            assert!(!is_valid_code_verifier(&verifier), "{verifier}");
        }
    }
}
