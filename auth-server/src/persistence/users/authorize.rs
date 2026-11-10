use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::Config, error::InternalError, security::issue_self_token::issue_self_token};

/// The user an identity-provider `authenticate` call vouched for.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthorizedUser {
    pub id: Uuid,
    pub has_temporary_password: bool,
}

#[derive(Debug)]
pub enum AuthorizeUserError {
    /// The user name is unknown or the password is wrong. The identity-provider tells these apart
    /// (404 / 401), but they are reported alike so callers can't leak which user names exist.
    InvalidCredentials,
    Internal(InternalError),
}

impl From<InternalError> for AuthorizeUserError {
    fn from(error: InternalError) -> Self {
        AuthorizeUserError::Internal(error)
    }
}

impl From<reqwest::Error> for AuthorizeUserError {
    fn from(error: reqwest::Error) -> Self {
        AuthorizeUserError::Internal(error.into())
    }
}

#[derive(Serialize)]
struct AuthenticateRequest<'a> {
    user_name: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct AuthenticateResponse {
    data: AuthorizedUser,
}

/// The `{"error": ..., "error_description": ...}` body of an identity-provider error response.
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Check a user name and password against the identity-provider (`POST /v1/users/authenticate`).
#[fnmock::mockable]
pub async fn authorize_user(
    user_name: &str,
    password: &str,
) -> Result<AuthorizedUser, AuthorizeUserError> {
    authorize_user_at(Config::identity_provider_url(), user_name, password).await
}

async fn authorize_user_at(
    base_url: &str,
    user_name: &str,
    password: &str,
) -> Result<AuthorizedUser, AuthorizeUserError> {
    let url = format!("{}/v1/users/authenticate", base_url.trim_end_matches('/'));
    tracing::debug!(%url, "calling identity-provider authenticate");

    let self_token = issue_self_token().await?;

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(self_token)
        .json(&AuthenticateRequest {
            user_name,
            password,
        })
        .send()
        .await?;
    let status = response.status();
    tracing::debug!(%status, "identity-provider authenticate responded");

    if status.is_success() {
        return Ok(response.json::<AuthenticateResponse>().await?.data);
    }

    let error = response.json::<ErrorResponse>().await?.error;
    match error.as_str() {
        "unauthorized" | "user_not_found" => {
            tracing::debug!(%status, %error, "identity-provider authenticate failed with invalid credentials");
            Err(AuthorizeUserError::InvalidCredentials)
        }
        _ => Err(InternalError::IdentityProviderResponse(format!(
            "authenticate failed with {status} {error}"
        ))
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode, header},
        routing::post,
    };
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;
    use crate::{logging::testing::LogCapture, security::issue_self_token::issue_self_token_fake};

    const SELF_TOKEN: &str = "the-self-token";

    /// The auth-server's own access token, without touching redis or postgres.
    fn fake_self_token() {
        issue_self_token_fake().setup(|| Ok(SELF_TOKEN.to_string()));
    }

    /// Serves `POST /v1/users/authenticate` with a fixed response and returns the base url.
    async fn identity_provider(status: StatusCode, body: Value) -> String {
        let app = Router::new().route(
            "/v1/users/authenticate",
            post(move |headers: HeaderMap, Json(request): Json<Value>| async move {
                // Check what was sent: the self token and the credentials.
                if status.is_success() {
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        &format!("Bearer {SELF_TOKEN}")
                    );
                    assert_eq!(request, json!({"user_name": "alice", "password": "secret"}));
                }
                (status, Json(body))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn returns_the_user_when_the_credentials_are_valid() {
        fake_self_token();
        let id = Uuid::new_v4();
        let base_url = identity_provider(
            StatusCode::OK,
            json!({"data": {
                "id": id, "user_name": "alice", "display_name": "Alice",
                "has_temporary_password": true, "roles": []
            }}),
        )
        .await;

        let user = authorize_user_at(&base_url, "alice", "secret").await;

        assert_eq!(
            user.ok(),
            Some(AuthorizedUser {
                id,
                has_temporary_password: true
            })
        );
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_password_is_wrong() {
        fake_self_token();
        let base_url = identity_provider(
            StatusCode::UNAUTHORIZED,
            json!({"error": "unauthorized", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "wrong").await;

        assert!(matches!(
            result,
            Err(AuthorizeUserError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_user_does_not_exist() {
        fake_self_token();
        let base_url = identity_provider(
            StatusCode::NOT_FOUND,
            json!({"error": "user_not_found", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "nobody", "secret").await;

        assert!(matches!(
            result,
            Err(AuthorizeUserError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_a_credentials_status_has_another_error_code() {
        fake_self_token();
        let base_url = identity_provider(
            StatusCode::UNAUTHORIZED,
            json!({"error": "invalid_request", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        let Err(AuthorizeUserError::Internal(error)) = result else {
            panic!("expected an internal error");
        };
        assert!(error.to_string().contains("invalid_request"), "{error}");
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_error_response_is_malformed() {
        fake_self_token();
        let base_url =
            identity_provider(StatusCode::UNAUTHORIZED, json!({"message": "nope"})).await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        assert!(matches!(
            result,
            Err(AuthorizeUserError::Internal(
                InternalError::IdentityProviderRequest(_)
            ))
        ));
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_identity_provider_fails() {
        fake_self_token();
        let base_url = identity_provider(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "internal_server_error", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        let Err(AuthorizeUserError::Internal(error)) = result else {
            panic!("expected an internal error");
        };
        let message = error.to_string();
        assert!(message.contains("500"), "{message}");
        assert!(message.contains("internal_server_error"), "{message}");
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_identity_provider_is_unreachable() {
        fake_self_token();
        // Nothing listens on port 1.
        let result = authorize_user_at("http://127.0.0.1:1", "alice", "secret").await;

        assert!(matches!(
            result,
            Err(AuthorizeUserError::Internal(
                InternalError::IdentityProviderRequest(_)
            ))
        ));
    }

    #[tokio::test]
    async fn logs_the_call_to_the_identity_provider_without_the_credentials() {
        fake_self_token();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let base_url = identity_provider(
            StatusCode::OK,
            json!({"data": {
                "id": Uuid::new_v4(), "user_name": "alice", "display_name": "Alice",
                "has_temporary_password": false, "roles": []
            }}),
        )
        .await;

        authorize_user_at(&base_url, "alice", "secret")
            .await
            .unwrap();

        let log = capture.contents();
        assert!(
            log.contains("calling identity-provider authenticate"),
            "{log}"
        );
        assert!(log.contains("/v1/users/authenticate"), "{log}");
        assert!(
            log.contains("identity-provider authenticate responded"),
            "{log}"
        );
        assert!(!log.contains("secret"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }
}
