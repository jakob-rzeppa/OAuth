use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::error::InternalError;

/// The errors of a protected resource (RFC 6750, section 3). Unlike the OAuth endpoints, they are
/// reported in the `WWW-Authenticate` header. The body repeats `error` and `error_description` as
/// JSON for clients that don't parse the header.
#[derive(Debug)]
pub enum UserInfoErrorResponse {
    /// No bearer token was sent. Only the authentication scheme is named, without an error code
    /// and with an empty body (RFC 6750, section 3.1).
    MissingToken,
    /// The token is unknown or expired, or its user no longer exists.
    InvalidToken,
    /// The token was not issued for OpenID Connect (no `openid` scope).
    InsufficientScope,
    ServerError(InternalError),
}

impl From<InternalError> for UserInfoErrorResponse {
    fn from(error: InternalError) -> Self {
        UserInfoErrorResponse::ServerError(error)
    }
}

impl IntoResponse for UserInfoErrorResponse {
    fn into_response(self) -> Response {
        let (status, error, description, extra) = match self {
            UserInfoErrorResponse::MissingToken => {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
                )
                    .into_response();
            }
            UserInfoErrorResponse::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "The access token is invalid or expired.",
                "",
            ),
            UserInfoErrorResponse::InsufficientScope => (
                StatusCode::FORBIDDEN,
                "insufficient_scope",
                "The access token was not granted the openid scope.",
                r#", scope="openid""#,
            ),
            UserInfoErrorResponse::ServerError(source) => {
                tracing::error!(error = ?source, "request failed with a server error");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "server_error",
                        "error_description": "An unexpected error occurred while processing the request.",
                    })),
                )
                    .into_response();
            }
        };

        let challenge =
            format!(r#"Bearer error="{error}", error_description="{description}"{extra}"#);

        (
            status,
            [(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_str(&challenge)
                    .expect("the challenge is built from ASCII constants"),
            )],
            Json(json!({
                "error": error,
                "error_description": description,
            })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::testing::LogCapture;

    async fn body(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    fn challenge(response: &Response) -> &str {
        response
            .headers()
            .get(header::WWW_AUTHENTICATE)
            .unwrap()
            .to_str()
            .unwrap()
    }

    #[tokio::test]
    async fn a_missing_token_only_names_the_scheme_without_a_body() {
        let response = UserInfoErrorResponse::MissingToken.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(challenge(&response), "Bearer");
        assert_eq!(body(response).await, "");
    }

    #[tokio::test]
    async fn an_invalid_token_is_unauthorized_with_invalid_token() {
        let response = UserInfoErrorResponse::InvalidToken.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            challenge(&response),
            r#"Bearer error="invalid_token", error_description="The access token is invalid or expired.""#
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body(response).await).unwrap(),
            json!({
                "error": "invalid_token",
                "error_description": "The access token is invalid or expired.",
            })
        );
    }

    #[tokio::test]
    async fn insufficient_scope_is_forbidden_and_names_the_required_scope() {
        let response = UserInfoErrorResponse::InsufficientScope.into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            challenge(&response),
            r#"Bearer error="insufficient_scope", error_description="The access token was not granted the openid scope.", scope="openid""#
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body(response).await).unwrap(),
            json!({
                "error": "insufficient_scope",
                "error_description": "The access token was not granted the openid scope.",
            })
        );
    }

    #[tokio::test]
    async fn logs_a_server_error_without_a_challenge() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        let response =
            UserInfoErrorResponse::ServerError(InternalError::Invariant("postgres is down"))
                .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(header::WWW_AUTHENTICATE).is_none());
        let body = body(response).await;
        assert!(body.contains(r#""error":"server_error""#), "{body}");
        assert!(!body.contains("postgres is down"), "{body}");
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("postgres is down"), "{log}");
    }
}
