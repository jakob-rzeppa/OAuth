use argon2::{Argon2, password_hash::PasswordHasher};

use crate::config::CONFIG;

pub enum HashPasswordError {
    HashingError,
}

#[fnmock::fakeable]
pub fn hash_password(password: &str) -> Result<String, HashPasswordError> {
    let argon2 = Argon2::new_with_secret(
        CONFIG.database_pepper().as_bytes(),
        argon2::Algorithm::default(),
        argon2::Version::default(),
        argon2::Params::default(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Initializing Argon2 failed");
        HashPasswordError::HashingError
    })?;

    argon2
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| {
            tracing::error!(error = %e, "Hashing a password failed");
            HashPasswordError::HashingError
        })
}
