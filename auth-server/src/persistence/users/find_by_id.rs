use serde::Deserialize;
use uuid::Uuid;

use crate::{config::Config, error::InternalError, security::issue_self_token::issue_self_token};

/// The profile data of a user, as far as the auth-server needs it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub user_name: String,
    pub display_name: String,
}

#[derive(Deserialize)]
struct GetUserResponse {
    data: User,
}

/// The `{"error": ..., "error_description": ...}` body of an identity-provider error response.
#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Looks a user up at the identity-provider (`GET /v1/users/{user_id}`).
/// Returns `None` if no user with this id exists (anymore).
#[fnmock::mockable]
pub async fn find_user_by_id(user_id: Uuid) -> Result<Option<User>, InternalError> {
    find_user_by_id_at(Config::identity_provider_url(), user_id).await
}

async fn find_user_by_id_at(base_url: &str, user_id: Uuid) -> Result<Option<User>, InternalError> {
    let url = format!("{}/v1/users/{user_id}", base_url.trim_end_matches('/'));
    tracing::debug!(%url, "calling identity-provider get user");

    let self_token = issue_self_token().await?;

    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(self_token)
        .send()
        .await?;
    let status = response.status();
    tracing::debug!(%status, "identity-provider get user responded");

    if status.is_success() {
        return Ok(Some(response.json::<GetUserResponse>().await?.data));
    }

    let Ok(error) = response.json::<ErrorResponse>().await else {
        return Err(InternalError::IdentityProviderResponse(format!(
            "get user failed with {status}"
        )));
    };
    match error.error.as_str() {
        "user_not_found" => Ok(None),
        _ => Err(InternalError::IdentityProviderResponse(format!(
            "get user failed with {} {}",
            status, error.error
        ))),
    }
}

#[cfg(test)]
mod tests {
    use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
    use serde_json::{Value, json};
    use tokio::net::TcpListener;

    use super::*;

    /// Serves `GET /v1/users/{user_id}` with a fixed response and returns the base url.
    async fn identity_provider(status: StatusCode, body: Value) -> String {
        let app = Router::new().route(
            "/v1/users/{user_id}",
            get(move |Path(_): Path<String>| async move { (status, Json(body)) }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn returns_the_user() {
        let id = Uuid::new_v4();
        let base_url = identity_provider(
            StatusCode::OK,
            json!({"data": {
                "id": id, "user_name": "alice", "display_name": "Alice",
                "has_temporary_password": false, "roles": []
            }}),
        )
        .await;

        let user = find_user_by_id_at(&base_url, id).await.unwrap();

        assert_eq!(
            user,
            Some(User {
                id,
                user_name: "alice".to_string(),
                display_name: "Alice".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn returns_none_for_an_unknown_user() {
        let base_url = identity_provider(
            StatusCode::NOT_FOUND,
            json!({"error": "user_not_found", "error_description": "x"}),
        )
        .await;

        let user = find_user_by_id_at(&base_url, Uuid::new_v4()).await.unwrap();

        assert_eq!(user, None);
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_identity_provider_fails() {
        let base_url = identity_provider(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "internal_server_error", "error_description": "x"}),
        )
        .await;

        let Err(error) = find_user_by_id_at(&base_url, Uuid::new_v4()).await else {
            panic!("expected an internal error");
        };
        let message = error.to_string();
        assert!(message.contains("500"), "{message}");
        assert!(message.contains("internal_server_error"), "{message}");
    }

    #[tokio::test]
    async fn fails_with_internal_error_when_the_identity_provider_is_unreachable() {
        // Nothing listens on port 1.
        let result = find_user_by_id_at("http://127.0.0.1:1", Uuid::new_v4()).await;

        assert!(matches!(
            result,
            Err(InternalError::IdentityProviderRequest(_))
        ));
    }
}
