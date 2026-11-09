//! PKCE code verifier and challenge (RFC 7636).

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Number of random bytes in a generated verifier. RFC 7636 section 4.1 recommends 32 octets,
/// which base64url encode to 43 characters, the shortest verifier allowed.
const VERIFIER_BYTES: usize = 32;

pub struct CodeVerifier {
    value: String,
}

impl CodeVerifier {
    /// Generates a new verifier: 32 random octets from the OS's cryptographically secure
    /// generator, base64url encoded without padding (RFC 7636 section 4.1). The result is 43
    /// characters, all from the unreserved set `[A-Z] [a-z] [0-9] - _`.
    pub fn generate() -> Self {
        let mut bytes = [0u8; VERIFIER_BYTES];
        // Only fails when the OS has no random source, then the client can't run securely.
        getrandom::fill(&mut bytes).expect("the OS random number generator failed");
        Self { value: URL_SAFE_NO_PAD.encode(bytes) }
    }

    /// The verifier itself, sent with the token request.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The S256 challenge sent with the authorization request:
    /// `BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))`, without padding (RFC 7636 section 4.2).
    pub fn code_challenge(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.value.as_bytes()))
    }

    pub fn code_challenge_method(&self) -> &'static str {
        "S256"
    }
}
