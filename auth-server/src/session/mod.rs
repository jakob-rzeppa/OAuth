//! Handling of sessions that are identified by a token in a cookie and stored in redis under the
//! hash of that token. A kind of session (login, user, ...) only implements [`Session`]; issuing,
//! looking up and clearing it is shared.

pub mod cookie;
mod kinds;
mod token;

use axum::http::HeaderValue;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    error::InternalError,
    persistence::sessions::{access::access_session, save::save_session, take::take_session},
    util::token::{hash_token, random_token},
};

pub use token::SessionToken;

/// A kind of session. It is stored as JSON under `{KEY_PREFIX}:{hash of the session token}` and
/// handed to the browser as the cookie `COOKIE_NAME`.
pub trait Session: Serialize + DeserializeOwned {
    /// Has to carry the `__Host-` prefix, which keeps sibling subdomains from shadowing it.
    const COOKIE_NAME: &'static str;
    const KEY_PREFIX: &'static str;

    /// How long the session lives, in seconds.
    fn ttl_seconds() -> u64;
}

fn key<S: Session>(token: &str) -> String {
    format!("{}:{}", S::KEY_PREFIX, hash_token(token))
}

/// Starts a session: stores it under a fresh token and returns the `Set-Cookie` value handing
/// that token to the browser.
pub async fn issue<S: Session>(session: &S) -> Result<HeaderValue, InternalError> {
    let token = random_token();
    let ttl_seconds = S::ttl_seconds();

    save_session(
        &key::<S>(&token),
        &serde_json::to_string(session)?,
        ttl_seconds,
    )
    .await?;

    cookie::session_cookie(S::COOKIE_NAME, &token, ttl_seconds).ok_or(InternalError::Invariant(
        "the session cookie is not a valid header value",
    ))
}

/// Reads a session and renews its lifetime (sliding expiration).
/// Returns `None` if the session does not exist or has expired.
pub async fn access<S: Session>(token: &SessionToken<S>) -> Result<Option<S>, InternalError> {
    let value = access_session(&key::<S>(token.as_str()), S::ttl_seconds()).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}

/// Reads a session and deletes it, so it can only be used once.
/// Returns `None` if the session does not exist or has expired.
pub async fn take<S: Session>(token: &SessionToken<S>) -> Result<Option<S>, InternalError> {
    let value = take_session(&key::<S>(token.as_str())).await?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?)
}

/// The `Set-Cookie` value telling the browser to delete the cookie of the session kind.
pub fn clear_cookie<S: Session>() -> Result<HeaderValue, InternalError> {
    cookie::expired_cookie(S::COOKIE_NAME).ok_or(InternalError::Invariant(
        "the expired session cookie is not a valid header value",
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        config::Config,
        domain::entity::{login_session::LoginSession, user_session::UserSession},
        persistence::sessions::{access::access_session_fake, save::save_session_mock},
        util::token::random_token_fake,
    };

    use crate::persistence::sessions::take::take_session_fake;

    fn token<S: Session>(token: &str) -> SessionToken<S> {
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

        let session = access(&token::<UserSession>("token-123")).await.unwrap();

        assert_eq!(session, Some(UserSession::new(uuid::Uuid::nil())));
    }

    #[tokio::test]
    async fn access_returns_none_for_an_unknown_session() {
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(|_, _| Ok(None));

        let session = access(&token::<UserSession>("token-123")).await.unwrap();

        assert_eq!(session, None);
    }

    #[tokio::test]
    async fn take_consumes_the_session_under_the_token_hash() {
        take_session_fake().setup(|key| {
            assert_eq!(key, format!("session:login:{}", hash_token("token-123")));
            Ok(Some(
                serde_json::to_string(&LoginSession::new("csrf".to_string())).unwrap(),
            ))
        });

        let session = take(&token::<LoginSession>("token-123")).await.unwrap();

        assert_eq!(session, Some(LoginSession::new("csrf".to_string())));
    }

    #[tokio::test]
    async fn take_fails_when_the_stored_value_is_not_a_session() {
        take_session_fake().setup(|_| Ok(Some("not json".to_string())));

        let session = take(&token::<LoginSession>("token-123")).await;

        assert!(session.is_err());
    }

    #[test]
    fn clear_cookie_expires_the_cookie_of_the_session_kind() {
        assert_eq!(
            clear_cookie::<LoginSession>().unwrap(),
            "__Host-login_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }
}
