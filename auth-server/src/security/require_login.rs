use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    domain::user_session::UserSession,
    error::InternalError,
    session::{self, SessionToken},
};

/// Redirects the user to the login and back to the authorization request afterwards.
pub struct LoginRedirect {
    pub client_id: Uuid,
    pub request_uri: String,
}

impl IntoResponse for LoginRedirect {
    fn into_response(self) -> Response {
        let return_to = format!(
            "/authorize?client_id={}&request_uri={}",
            self.client_id, self.request_uri
        );
        match serde_urlencoded::to_string([("return_to", return_to)]) {
            Ok(query) => Redirect::to(&format!("/login?{query}")).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub enum RequireLoginError {
    LoginRequired(LoginRedirect),
    Internal(InternalError),
}

impl From<InternalError> for RequireLoginError {
    fn from(error: InternalError) -> Self {
        RequireLoginError::Internal(error)
    }
}

/// Make sure the user is logged in and has a valid session (which ttl is renewed by this call).
/// If not, the error carries a redirect to the login page.
pub async fn require_login(
    session_token: Option<SessionToken>,
    client_id: Uuid,
    request_uri: &str,
) -> Result<UserSession, RequireLoginError> {
    let login_required = || {
        RequireLoginError::LoginRequired(LoginRedirect {
            client_id,
            request_uri: request_uri.to_string(),
        })
    };

    let session_token = session_token.ok_or_else(login_required)?;

    session::access(&session_token).await?.ok_or_else(|| {
        tracing::warn!("Expired or invalid user session token was received.");
        login_required()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, logging::testing::LogCapture,
        persistence::user_sessions::access::access_session_fake,
    };

    const REQUEST_URI: &str = "urn:authorize:request_uri:test";

    #[derive(serde::Deserialize)]
    struct LoginQuery {
        return_to: String,
    }

    fn token() -> Option<SessionToken> {
        Some(SessionToken::new("session-token".to_string()))
    }

    #[tokio::test]
    async fn returns_the_session_for_a_valid_token() {
        let user_id = Uuid::new_v4();
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(move |_, _| {
            Ok(Some(
                serde_json::to_string(&UserSession::new(user_id)).unwrap(),
            ))
        });

        let result = require_login(token(), Uuid::new_v4(), REQUEST_URI).await;

        assert_eq!(result.ok().unwrap().user_id(), user_id);
    }

    #[tokio::test]
    async fn requires_login_without_a_session_token() {
        let client_id = Uuid::new_v4();

        let result = require_login(None, client_id, REQUEST_URI).await;

        let Err(RequireLoginError::LoginRequired(redirect)) = result else {
            panic!("expected a login required error");
        };
        assert_eq!(redirect.client_id, client_id);
        assert_eq!(redirect.request_uri, REQUEST_URI);
    }

    #[tokio::test]
    async fn requires_login_and_logs_a_warning_when_the_session_is_expired() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(|_, _| Ok(None));

        let result = require_login(token(), Uuid::new_v4(), REQUEST_URI).await;

        assert!(matches!(result, Err(RequireLoginError::LoginRequired(_))));
        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(
            log.contains("Expired or invalid user session token"),
            "{log}"
        );
        assert!(!log.contains("session-token"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_an_internal_error_when_access_session_fails() {
        Config::user_session_ttl_fake().setup(|| 1800);
        access_session_fake().setup(|_, _| Err(InternalError::Invariant("redis is down")));

        let result = require_login(token(), Uuid::new_v4(), REQUEST_URI).await;

        assert!(matches!(result, Err(RequireLoginError::Internal(_))));
    }

    #[test]
    fn login_redirect_points_to_login_with_the_authorization_as_return_to() {
        let client_id = Uuid::new_v4();

        let response = LoginRedirect {
            client_id,
            request_uri: REQUEST_URI.to_string(),
        }
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        let location = response.headers()[axum::http::header::LOCATION]
            .to_str()
            .unwrap();
        let query = location.strip_prefix("/login?").unwrap();
        let LoginQuery { return_to } = serde_urlencoded::from_str(query).unwrap();
        assert_eq!(
            return_to,
            format!("/authorize?client_id={client_id}&request_uri={REQUEST_URI}")
        );
    }
}
