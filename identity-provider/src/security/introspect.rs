use serde::Deserialize;
use uuid::Uuid;

use crate::config::CONFIG;

#[derive(Debug, thiserror::Error)]
pub enum IntrospectionError {
    /// The auth-server could not be reached, or answered with something unexpected.
    #[error("request to auth-server failed: {0}")]
    Request(reqwest::Error),
}

impl From<reqwest::Error> for IntrospectionError {
    fn from(error: reqwest::Error) -> Self {
        IntrospectionError::Request(error)
    }
}

#[derive(Debug)]
pub enum Introspection {
    Active {
        scope: String,
        client_id: Uuid,
        sub: Option<Uuid>,
    },
    Inactive,
}

#[derive(Deserialize)]
struct IntrospectionResponse {
    active: bool,
    scope: Option<String>,
    client_id: Option<Uuid>,
    sub: Option<Uuid>,
}

/// Asks the auth-server (RFC 7662) whether `token` is an active access token.
pub async fn introspect_token(token: &str) -> Result<Introspection, IntrospectionError> {
    introspect_token_at(CONFIG.auth_server_url(), token).await
}

async fn introspect_token_at(
    base_url: &str,
    token: &str,
) -> Result<Introspection, IntrospectionError> {
    let url = format!("{}/introspect", base_url.trim_end_matches('/'));
    tracing::debug!(%url, "calling auth-server introspect");

    let response = reqwest::Client::new()
        .post(url)
        .form(&[("token", token)])
        .send()
        .await?
        .error_for_status()?;

    let body = response.json::<IntrospectionResponse>().await?;

    match body {
        IntrospectionResponse {
            active: true,
            scope: Some(scope),
            client_id: Some(client_id),
            sub,
        } => Ok(Introspection::Active {
            scope,
            client_id,
            sub,
        }),
        _ => Ok(Introspection::Inactive),
    }
}
