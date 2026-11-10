use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::Config, error::InternalError, identity_provider::self_token::issue_self_token,
};

#[derive(Debug)]
pub enum SetUserPasswordError {
    /// The current password is wrong, or the user does not exist (anymore).
    InvalidCredentials,
    Internal(InternalError),
}

impl From<InternalError> for SetUserPasswordError {
    fn from(error: InternalError) -> Self {
        SetUserPasswordError::Internal(error)
    }
}

impl From<reqwest::Error> for SetUserPasswordError {
    fn from(error: reqwest::Error) -> Self {
        SetUserPasswordError::Internal(error.into())
    }
}

#[derive(Serialize)]
struct SetPasswordRequest<'a> {
    current_password: &'a str,
    new_password: &'a str,
}

/// The `{"error": ..., "error_description": ...}` body of an identity-provider error response.
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Replaces the user's password at the identity-provider (`PUT /v1/users/{user_id}/password`),
/// which also ends a temporary password.
#[fnmock::mockable]
pub async fn set_user_password(
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), SetUserPasswordError> {
    set_user_password_at(
        Config::identity_provider_url(),
        user_id,
        current_password,
        new_password,
    )
    .await
}

async fn set_user_password_at(
    base_url: &str,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), SetUserPasswordError> {
    let url = format!(
        "{}/v1/users/{user_id}/password",
        base_url.trim_end_matches('/')
    );
    tracing::debug!(%url, "calling identity-provider set password");

    let self_token = issue_self_token().await?;

    let response = reqwest::Client::new()
        .put(url)
        .bearer_auth(self_token)
        .json(&SetPasswordRequest {
            current_password,
            new_password,
        })
        .send()
        .await?;
    let status = response.status();
    tracing::debug!(%status, "identity-provider set password responded");

    if status.is_success() {
        return Ok(());
    }

    let error = response.json::<ErrorResponse>().await?.error;
    match error.as_str() {
        "invalid_credentials" | "user_not_found" => Err(SetUserPasswordError::InvalidCredentials),
        _ => Err(InternalError::IdentityProviderResponse(format!(
            "set password failed with {status} {error}"
        ))
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        extract::Path,
        http::{HeaderMap, StatusCode, header},
        response::{IntoResponse, Response},
        routing::put,
    };
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;
    use crate::{
        identity_provider::self_token::issue_self_token_fake, logging::testing::LogCapture,
    };

    const SELF_TOKEN: &str = "the-self-token";

    fn fake_self_token() {
        issue_self_token_fake().setup(|| Ok(SELF_TOKEN.to_string()));
    }

    /// Serves `PUT /v1/users/{user_id}/password` with a fixed response and returns the base url.
    /// A `None` body answers with an empty body, like the identity-provider's 204.
    async fn identity_provider(user_id: Uuid, status: StatusCode, body: Option<Value>) -> String {
        let app = Router::new().route(
            "/v1/users/{user_id}/password",
            put(
                move |headers: HeaderMap,
                      Path(path_user_id): Path<Uuid>,
                      Json(request): Json<Value>| async move {
                    assert_eq!(path_user_id, user_id);
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        &format!("Bearer {SELF_TOKEN}")
                    );
                    assert_eq!(
                        request,
                        json!({"current_password": "temporary", "new_password": "chosen"})
                    );
                    let response: Response = match body {
                        Some(body) => (status, Json(body)).into_response(),
                        None => status.into_response(),
                    };
                    response
                },
            ),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn succeeds_when_the_identity_provider_accepts_the_new_password() {
        fake_self_token();
        let user_id = Uuid::new_v4();
        let base_url = identity_provider(user_id, StatusCode::NO_CONTENT, None).await;

        let result = set_user_password_at(&base_url, user_id, "temporary", "chosen").await;

        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_current_password_is_wrong() {
        fake_self_token();
        let user_id = Uuid::new_v4();
        let base_url = identity_provider(
            user_id,
            StatusCode::UNAUTHORIZED,
            Some(json!({"error": "invalid_credentials", "error_description": "x"})),
        )
        .await;

        let result = set_user_password_at(&base_url, user_id, "temporary", "chosen").await;

        assert!(matches!(
            result,
            Err(SetUserPasswordError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn fails_with_invalid_credentials_when_the_user_does_not_exist() {
        fake_self_token();
        let user_id = Uuid::new_v4();
        let base_url = identity_provider(
            user_id,
            StatusCode::NOT_FOUND,
            Some(json!({"error": "user_not_found", "error_description": "x"})),
        )
        .await;

        let result = set_user_password_at(&base_url, user_id, "temporary", "chosen").await;

        assert!(matches!(
            result,
            Err(SetUserPasswordError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_identity_provider_fails() {
        fake_self_token();
        let user_id = Uuid::new_v4();
        let base_url = identity_provider(
            user_id,
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(json!({"error": "internal_server_error", "error_description": "x"})),
        )
        .await;

        let result = set_user_password_at(&base_url, user_id, "temporary", "chosen").await;

        let Err(SetUserPasswordError::Internal(error)) = result else {
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
        let result =
            set_user_password_at("http://127.0.0.1:1", Uuid::new_v4(), "temporary", "chosen").await;

        assert!(matches!(
            result,
            Err(SetUserPasswordError::Internal(
                InternalError::IdentityProviderRequest(_)
            ))
        ));
    }

    #[tokio::test]
    async fn logs_the_call_without_the_passwords() {
        fake_self_token();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let user_id = Uuid::new_v4();
        let base_url = identity_provider(user_id, StatusCode::NO_CONTENT, None).await;

        set_user_password_at(&base_url, user_id, "temporary", "chosen")
            .await
            .unwrap();

        let log = capture.contents();
        assert!(
            log.contains("calling identity-provider set password"),
            "{log}"
        );
        assert!(
            log.contains("identity-provider set password responded"),
            "{log}"
        );
        assert!(!log.contains("temporary"), "{log}");
        assert!(!log.contains("chosen"), "{log}");
    }
}
