//! Handling of the user session. It is identified by a token in a cookie and stored in redis
//! under the hash of that token.

pub mod cookie;
mod token;

use axum::http::HeaderValue;

use crate::{
    config::Config,
    domain::user_session::UserSession,
    error::InternalError,
    persistence::user_sessions::{access::access_session, save::save_session},
    util::token::{hash_token, random_token},
};

pub use token::SessionToken;

/// Has to carry the `__Host-` prefix, which keeps sibling subdomains from shadowing it.
const COOKIE_NAME: &str = "__Host-user_session";

/// The session is stored as JSON under `{KEY_PREFIX}:{hash of the session token}`.
const KEY_PREFIX: &str = "session:user";

fn key(token: &str) -> String {
    format!("{KEY_PREFIX}:{}", hash_token(token))
}

/// Starts a session: stores it under a fresh token and returns the `Set-Cookie` value handing
/// that token to the browser.
pub async fn issue(session: &UserSession) -> Result<HeaderValue, InternalError> {
    let token = random_token();
    let ttl_seconds = Config::user_session_ttl();

    save_session(&key(&token), &serde_json::to_string(session)?, ttl_seconds).await?;

    cookie::session_cookie(COOKIE_NAME, &token, ttl_seconds).ok_or(InternalError::Invariant(
        "the session cookie is not a valid header value",
    ))
}

/// Reads a session and renews its lifetime (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
pub async fn access(token: &SessionToken) -> Result<Option<UserSession>, InternalError> {
    let value = access_session(&key(token.as_str()), Config::user_session_ttl()).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        config::Config,
        domain::user_session::UserSession,
        persistence::user_sessions::{access::access_session_fake, save::save_session_mock},
        util::token::random_token_fake,
    };

    fn token(token: &str) -> SessionToken {
        SessionToken::new(token.to_string())
    }

    #[tokio::test]
    async fn issue_stores_the_session_under_the_token_hash_and_returns_its_cookie() {
        let saved = Arc::new(Mutex::new(None));
        let saved_in_mock = saved.clone();
        random_token_fake().setup(|| "token-123".to_string());
        Config::user_session_ttl_fake().setup(|| 1800);
        save_session_mock().setup(move |key, value, ttl_seconds| {
            *saved_in_mock.lock().unwrap() =
                Some((key.to_string(), value.to_string(), ttl_seconds));
            Ok(())
        });
        let user_session = UserSession::new(uuid::Uuid::new_v4());

        let cookie = issue(&user_session).await.unwrap();

        assert_eq!(
            cookie,
            "__Host-user_session=token-123; Max-Age=1800; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
        let (key, value, ttl_seconds) = saved.lock().unwrap().clone().unwrap();
        assert_eq!(key, format!("session:user:{}", hash_token("token-123")));
        assert_eq!(value, serde_json::to_string(&user_session).unwrap());
        assert_eq!(ttl_seconds, 1800);
    }

    #[tokio::test]
    async fn access_renews_the_session_under_the_token_hash() {
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(|key, ttl_seconds| {
            assert_eq!(key, format!("session:user:{}", hash_token("token-123")));
            assert_eq!(ttl_seconds, 1800);
            Ok(Some(
                serde_json::to_string(&UserSession::new(uuid::Uuid::nil())).unwrap(),
            ))
        });

        let session = access(&token("token-123")).await.unwrap();

        assert_eq!(session, Some(UserSession::new(uuid::Uuid::nil())));
    }

    #[tokio::test]
    async fn access_returns_none_for_an_unknown_session() {
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(|_, _| Ok(None));

        let session = access(&token("token-123")).await.unwrap();

        assert_eq!(session, None);
    }
}
