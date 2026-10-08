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
