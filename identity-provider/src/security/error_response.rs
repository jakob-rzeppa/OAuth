use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::security::introspect::IntrospectionError;

/// The ways the auth middleware can reject a request.
#[derive(Debug)]
pub enum AuthErrorResponse {
    /// No usable token was presented, or the auth-server reports it as inactive.
    Unauthorized,

    /// The token is valid, but not allowed to access this route.
    Forbidden,

    /// The token could not be checked, because the auth-server failed. This says nothing
    /// about the token, so the client must not discard it.
    ServiceUnavailable,
}

impl IntoResponse for AuthErrorResponse {
    /// Renders the rejection as `{"error": ..., "error_description": ...}`, like the endpoint
    /// errors. A 401 additionally carries `WWW-Authenticate: Bearer`, which tells the client
    /// to provide a Bearer token (RFC 6750).
    fn into_response(self) -> Response {
        let (status, error, description) = match self {
            AuthErrorResponse::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "A valid access token is required.",
            ),
            AuthErrorResponse::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "The access token is not allowed to access this resource.",
            ),
            AuthErrorResponse::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                "The access token could not be verified. Try again later.",
            ),
        };
        let body = Json(json!({ "error": error, "error_description": description }));

        match self {
            AuthErrorResponse::Unauthorized => {
                (status, [(header::WWW_AUTHENTICATE, "Bearer")], body).into_response()
            }
            _ => (status, body).into_response(),
        }
    }
}

impl From<IntrospectionError> for AuthErrorResponse {
    fn from(error: IntrospectionError) -> Self {
        tracing::error!("Token introspection failed unexpectedly: {}", error);
        AuthErrorResponse::ServiceUnavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_rejection_to_its_status() {
        let status = |error: AuthErrorResponse| error.into_response().status();

        assert_eq!(
            status(AuthErrorResponse::Unauthorized),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(status(AuthErrorResponse::Forbidden), StatusCode::FORBIDDEN);
        assert_eq!(
            status(AuthErrorResponse::ServiceUnavailable),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn only_the_unauthorized_response_carries_the_bearer_challenge() {
        let challenge = |error: AuthErrorResponse| {
            error
                .into_response()
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .cloned()
        };

        assert_eq!(
            challenge(AuthErrorResponse::Unauthorized).unwrap(),
            "Bearer"
        );
        assert!(challenge(AuthErrorResponse::Forbidden).is_none());
        assert!(challenge(AuthErrorResponse::ServiceUnavailable).is_none());
    }

    #[tokio::test]
    async fn renders_the_error_body() {
        let response = AuthErrorResponse::ServiceUnavailable.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            r#"{"error":"service_unavailable","error_description":"The access token could not be verified. Try again later."}"#
        );
    }
}
