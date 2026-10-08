use axum::{extract::OptionalFromRequestParts, http::HeaderValue};
use uuid::Uuid;

use crate::{
    config::Config,
    domain::entity::user_session::UserSession,
    error::InternalError,
    persistence::user_session::save::save_user_session,
    util::{
        cookie::{cookie_value, session_cookie},
        token::{hash_token, random_token},
    },
};

/// Name of the cookie carrying the user session token, set once the user is logged in.
const USER_SESSION_COOKIE: &str = "user_session";

pub struct UserSessionToken(String);

impl UserSessionToken {
    #[cfg(test)]
    pub fn new(token: String) -> Self {
        UserSessionToken(token)
    }

    pub fn token(&self) -> &str {
        &self.0
    }
}

impl OptionalFromRequestParts<()> for UserSessionToken {
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &(),
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts
            .headers
            .get_all(axum::http::header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|value| cookie_value(value, USER_SESSION_COOKIE))
            .next()
            .map(|token| UserSessionToken(token.to_string())))
    }
}

/// Creates a new user session for the given user ID, saves it to the database, and returns a cookie header value.
pub async fn create_user_session(user_id: Uuid) -> Result<HeaderValue, InternalError> {
    let session_token = random_token();
    let session_ttl = Config::user_session_ttl();
    let user_session_entity = UserSession::new(user_id);

    save_user_session(
        &hash_token(&session_token),
        user_session_entity,
        session_ttl,
    )
    .await?;

    session_cookie(USER_SESSION_COOKIE, &session_token, session_ttl).ok_or(
        InternalError::Invariant("the user session cookie is not a valid header value"),
    )
}
