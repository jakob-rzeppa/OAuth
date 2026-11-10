use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

/// Number of random bytes in a nonce: 256 bits, so it can't be guessed.
const NONCE_BYTES: usize = 32;

/// Binds the ID token to this authorization request (OpenID Connect Core 1.0 section 3.1.2.1):
/// sent with the request, echoed by the auth-server in the ID token's `nonce` claim. A token
/// issued for another request, e.g. a replayed one, carries a different nonce.
#[derive(Debug, Clone, PartialEq)]
pub struct Nonce {
    value: String,
}

impl Nonce {
    pub fn generate() -> Self {
        let mut bytes = [0u8; NONCE_BYTES];
        // Only fails when the OS has no random source, then the client can't run securely.
        getrandom::fill(&mut bytes).expect("the OS random number generator failed");
        Nonce { value: URL_SAFE_NO_PAD.encode(bytes) }
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
