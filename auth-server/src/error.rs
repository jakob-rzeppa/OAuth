//! The errors of the auth-server.
//!
//! An endpoint describes its errors in its own enum and turns each of them into one of the three
//! shapes a client can be answered with, which render, log and validate them the same way
//! everywhere:
//!
//! - [`api::ApiError`]: a JSON body, for the endpoints called by clients.
//! - [`page::PageError`]: the HTML error page, for the endpoints the user's browser visits.
//! - [`redirect::RedirectError`]: a redirect to the client's `redirect_uri`.

pub mod api;
pub mod page;
pub mod redirect;

/// Implements `IntoResponse` for an endpoint's error enum by converting it into one of the global
/// error types: `into_response_via!(TokenErrorResponse => ApiError);`.
macro_rules! into_response_via {
    ($error:ty => $via:ty) => {
        impl axum::response::IntoResponse for $error {
            fn into_response(self) -> axum::response::Response {
                axum::response::IntoResponse::into_response(<$via>::from(self))
            }
        }
    };
}
pub(crate) use into_response_via;

/// What every `server_error` tells the client. The cause is only logged.
pub const SERVER_ERROR_DESCRIPTION: &str =
    "An unexpected error occurred while processing the request.";

/// Failures of the server itself (infrastructure, broken invariants), as opposed to errors
/// caused by the request. They are never shown to the client: an endpoint turns them into a
/// generic `server_error` response and logs them there, once, with the request's context.
#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("redis: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("postgres: {0}")]
    Postgres(#[from] sqlx::Error),

    #[error("(de)serializing stored data failed: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("signing a JWT failed: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("identity-provider request failed: {0}")]
    IdentityProviderRequest(#[from] reqwest::Error),

    #[error("unexpected identity-provider response: {0}")]
    IdentityProviderResponse(String),

    /// Something the code relies on did not hold, e.g. a value that is always a valid
    /// header turned out not to be.
    #[error("{0}")]
    Invariant(&'static str),
}

impl InternalError {
    /// Logs the error behind a `server_error` response. Called once, when the response is built.
    pub fn log(&self) {
        tracing::error!(error = ?self, "request failed with a server error");
    }
}
