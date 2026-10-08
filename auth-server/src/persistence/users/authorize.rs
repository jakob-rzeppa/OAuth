use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

/// The user an identity-server `authenticate` call vouched for.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthorizedUser {
    pub id: Uuid,
    pub has_temporary_password: bool,
}

#[derive(Debug, PartialEq)]
pub enum AuthorizeUserError {
    /// The user name is unknown or the password is wrong. The identity-server tells these apart
    /// (404 / 401), but they are reported alike so callers can't leak which user names exist.
    InvalidCredentials,
    ServerError,
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

/// The `{"error": ..., "error_description": ...}` body of an identity-server error response.
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Check a user name and password against the identity-server (`POST /v1/users/authenticate`).
#[fnmock::mockable]
pub async fn authorize_user(
    user_name: &str,
    password: &str,
) -> Result<AuthorizedUser, AuthorizeUserError> {
    authorize_user_at(Config::identity_server_url(), user_name, password).await
}

async fn authorize_user_at(
    base_url: &str,
    user_name: &str,
    password: &str,
) -> Result<AuthorizedUser, AuthorizeUserError> {
    let url = format!("{}/v1/users/authenticate", base_url.trim_end_matches('/'));
    tracing::debug!(%url, "calling identity-server authenticate");

    let response = reqwest::Client::new()
        .post(url)
        .json(&AuthenticateRequest {
            user_name,
            password,
        })
        .send()
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to reach identity-server");
            AuthorizeUserError::ServerError
        })?;
    tracing::debug!(status = %response.status(), "identity-server authenticate responded");

    match response.status() {
        status if status.is_success() => response
            .json::<AuthenticateResponse>()
            .await
            .map(|response| response.data)
            .map_err(|error| {
                tracing::error!(?error, "Invalid identity-server response");
                AuthorizeUserError::ServerError
            }),
        status => {
            let error = response
                .json::<ErrorResponse>()
                .await
                .map(|response| response.error)
                .map_err(|error| {
                    tracing::error!(%status, ?error, "Invalid identity-server error response");
                    AuthorizeUserError::ServerError
                })?;
            match error.as_str() {
                "unauthorized" | "user_not_found" => {
                    tracing::debug!(%status, %error, "identity-server authenticate failed with invalid credentials");
                    Err(AuthorizeUserError::InvalidCredentials)
                }
                _ => {
                    tracing::error!(%status, %error, "identity-server authenticate failed");
                    Err(AuthorizeUserError::ServerError)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;
    use crate::logging::testing::LogCapture;

    /// Serves `POST /v1/users/authenticate` with a fixed response and returns the base url.
    async fn identity_server(status: StatusCode, body: Value) -> String {
        let app = Router::new().route(
            "/v1/users/authenticate",
            post(move |Json(request): Json<Value>| async move {
                // Echo the credentials back so tests can check what was sent.
                if status.is_success() {
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
        let id = Uuid::new_v4();
        let base_url = identity_server(
            StatusCode::OK,
            json!({"data": {
                "id": id, "user_name": "alice", "display_name": "Alice",
                "has_temporary_password": true, "roles": []
            }}),
        )
        .await;

        let user = authorize_user_at(&base_url, "alice", "secret").await;

        assert_eq!(
            user,
            Ok(AuthorizedUser {
                id,
                has_temporary_password: true
            })
        );
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_password_is_wrong() {
        let base_url = identity_server(
            StatusCode::UNAUTHORIZED,
            json!({"error": "unauthorized", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "wrong").await;

        assert_eq!(result, Err(AuthorizeUserError::InvalidCredentials));
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_user_does_not_exist() {
        let base_url = identity_server(
            StatusCode::NOT_FOUND,
            json!({"error": "user_not_found", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "nobody", "secret").await;

        assert_eq!(result, Err(AuthorizeUserError::InvalidCredentials));
    }

    #[tokio::test]
    async fn fails_with_server_error_when_a_credentials_status_has_another_error_code() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let base_url = identity_server(
            StatusCode::UNAUTHORIZED,
            json!({"error": "invalid_request", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        assert_eq!(result, Err(AuthorizeUserError::ServerError));
        let log = capture.contents();
        assert!(log.contains("invalid_request"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_server_error_when_the_error_response_is_malformed() {
        let base_url = identity_server(StatusCode::UNAUTHORIZED, json!({"message": "nope"})).await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        assert_eq!(result, Err(AuthorizeUserError::ServerError));
    }

    #[tokio::test]
    async fn fails_with_server_error_when_the_identity_server_fails() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let base_url = identity_server(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "internal_server_error", "error_description": "x"}),
        )
        .await;

        let result = authorize_user_at(&base_url, "alice", "secret").await;

        assert_eq!(result, Err(AuthorizeUserError::ServerError));
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("identity-server authenticate failed"), "{log}");
        assert!(log.contains("500"), "{log}");
        assert!(log.contains("internal_server_error"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_server_error_when_the_identity_server_is_unreachable() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        // Nothing listens on port 1.
        let result = authorize_user_at("http://127.0.0.1:1", "alice", "secret").await;

        assert_eq!(result, Err(AuthorizeUserError::ServerError));
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("Failed to reach identity-server"), "{log}");
    }

    #[tokio::test]
    async fn logs_the_call_to_the_identity_server_without_the_credentials() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let base_url = identity_server(
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
            log.contains("calling identity-server authenticate"),
            "{log}"
        );
        assert!(log.contains("/v1/users/authenticate"), "{log}");
        assert!(
            log.contains("identity-server authenticate responded"),
            "{log}"
        );
        assert!(!log.contains("secret"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }
}
