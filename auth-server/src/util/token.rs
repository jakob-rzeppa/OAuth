use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;
use sha2::Digest;

/// Generate a random 256-bit token and encode it in URL-safe base64.
/// Used for session tokens, authorization codes and access tokens.
#[fnmock::fakeable]
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash a token with SHA-256 and encode it in URL-safe base64.
/// Only the hash should be persisted, never the token itself.
pub fn hash_token(token: &str) -> String {
    let hash = sha2::Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_encodes_256_random_bits() {
        let bytes = URL_SAFE_NO_PAD.decode(random_token()).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn tokens_are_unique() {
        assert_ne!(random_token(), random_token());
    }

    #[test]
    fn hash_is_deterministic_sha256() {
        // SHA-256("abc"), base64url without padding
        assert_eq!(
            hash_token("abc"),
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
        );
    }

    #[test]
    fn hash_differs_from_token() {
        let token = random_token();
        assert_ne!(hash_token(&token), token);
    }
}
