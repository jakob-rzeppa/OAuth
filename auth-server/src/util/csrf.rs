use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;

/// Generate a random 256-bit CSRF token and encode it in URL-safe base64.
pub fn create_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_encodes_256_random_bits() {
        let token = create_csrf_token();
        let bytes = URL_SAFE_NO_PAD.decode(&token).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn tokens_are_unique() {
        assert_ne!(create_csrf_token(), create_csrf_token());
    }
}
