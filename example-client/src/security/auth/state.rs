use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    value: String,
}

impl PartialEq<&str> for State {
    fn eq(&self, other: &&str) -> bool {
        self.value == *other
    }
}

/// Number of random bytes in a state value: 256 bits, so it can't be guessed.
const STATE_BYTES: usize = 32;

impl State {
    pub fn generate() -> Self {
        let mut bytes = [0u8; STATE_BYTES];
        // Only fails when the OS has no random source, then the client can't run securely.
        getrandom::fill(&mut bytes).expect("the OS random number generator failed");
        State { value: URL_SAFE_NO_PAD.encode(bytes) }
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
