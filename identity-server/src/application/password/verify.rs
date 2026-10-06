use argon2::{Argon2, PasswordVerifier};

use crate::config::CONFIG;

pub enum VerifyPasswordError {
    InvalidPassword,
    HashingError,
}

#[fnmock::fakeable]
pub fn verify_password(password: &str, hash: &str) -> Result<(), VerifyPasswordError> {
    let argon2 = Argon2::new_with_secret(
        CONFIG.database_pepper().as_bytes(),
        argon2::Algorithm::default(),
        argon2::Version::default(),
        argon2::Params::default(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Initializing Argon2 failed");
        VerifyPasswordError::HashingError
    })?;

    argon2
        .verify_password(password.as_bytes(), hash)
        .map_err(|_| VerifyPasswordError::InvalidPassword)
}
