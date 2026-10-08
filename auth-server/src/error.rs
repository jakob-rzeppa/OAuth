//! Failures of the server itself (infrastructure, broken invariants), as opposed to errors
//! caused by the request. They are never shown to the client: an endpoint turns them into a
//! generic `server_error` response and logs them there, once, with the request's context.

#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("redis: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("postgres: {0}")]
    Postgres(#[from] sqlx::Error),

    #[error("(de)serializing stored data failed: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("identity-server request failed: {0}")]
    IdentityServerRequest(#[from] reqwest::Error),

    #[error("unexpected identity-server response: {0}")]
    IdentityServerResponse(String),

    /// Something the code relies on did not hold, e.g. a value that is always a valid
    /// header turned out not to be.
    #[error("{0}")]
    Invariant(&'static str),
}
