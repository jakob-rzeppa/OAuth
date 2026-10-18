use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{
    error::{InternalError, SERVER_ERROR_DESCRIPTION},
    util::oauth_error::OAuthErrorCode,
};

/// An error answered with a JSON body, for the endpoints called by clients.
#[derive(Debug)]
pub enum ApiError {
    /// `error` and `error_description` in the body, with the status of the code.
    Described {
        code: OAuthErrorCode,
        description: &'static str,
        /// Set for protected resources: the error is repeated in the `WWW-Authenticate` header
        /// (RFC 6750 §3).
        challenge: Option<Challenge>,
    },
    /// No credentials were sent to a protected resource: a 401 that only names the scheme,
    /// without an error code and with an empty body (RFC 6750 §3.1).
    Unauthenticated(Scheme),
    /// Logged, and answered with a generic `server_error`.
    Server(InternalError),
}

/// The authentication scheme a `WWW-Authenticate` challenge names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Bearer,
}

impl Scheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scheme::Bearer => "Bearer",
        }
    }
}

/// The `WWW-Authenticate` challenge a described error is repeated in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Challenge {
    pub scheme: Scheme,
    /// The scope the resource requires, for `insufficient_scope` (RFC 6750 §3).
    pub scope: Option<&'static str>,
}

impl Challenge {
    pub fn bearer() -> Self {
        Challenge {
            scheme: Scheme::Bearer,
            scope: None,
        }
    }

    pub fn with_scope(self, scope: &'static str) -> Self {
        Challenge {
            scope: Some(scope),
            ..self
        }
    }
}

impl ApiError {
    /// A described error without a challenge, as the OAuth endpoints answer.
    pub fn new(code: OAuthErrorCode, description: &'static str) -> Self {
        ApiError::Described {
            code,
            description,
            challenge: None,
        }
    }

    /// A described error of a protected resource, repeated in a `WWW-Authenticate` challenge.
    pub fn challenged(
        code: OAuthErrorCode,
        description: &'static str,
        challenge: Challenge,
    ) -> Self {
        ApiError::Described {
            code,
            description,
            challenge: Some(challenge),
        }
    }
}

impl From<InternalError> for ApiError {
    fn from(error: InternalError) -> Self {
        ApiError::Server(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Described {
                code,
                description,
                challenge,
            } => {
                let body = body(code.as_str(), description);
                let Some(challenge) = challenge else {
                    return (code.status(), body).into_response();
                };

                let Ok(value) =
                    HeaderValue::from_str(&described_challenge(challenge, code, description))
                else {
                    return ApiError::Server(InternalError::Invariant(
                        "an error challenge is not a valid header value",
                    ))
                    .into_response();
                };
                (code.status(), [(header::WWW_AUTHENTICATE, value)], body).into_response()
            }
            ApiError::Unauthenticated(scheme) => (
                StatusCode::UNAUTHORIZED,
                [(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static(scheme.as_str()),
                )],
            )
                .into_response(),
            ApiError::Server(source) => {
                source.log();
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    body(
                        OAuthErrorCode::ServerError.as_str(),
                        SERVER_ERROR_DESCRIPTION,
                    ),
                )
                    .into_response()
            }
        }
    }
}

fn body(error: &str, description: &str) -> Json<serde_json::Value> {
    Json(json!({
        "error": error,
        "error_description": description,
    }))
}

fn described_challenge(challenge: Challenge, code: OAuthErrorCode, description: &str) -> String {
    let mut value = format!(
        r#"{} error="{}", error_description="{description}""#,
        challenge.scheme.as_str(),
        code.as_str(),
    );
    if let Some(scope) = challenge.scope {
        value.push_str(&format!(r#", scope="{scope}""#));
    }
    value
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

    async fn json_body(response: Response) -> serde_json::Value {
        serde_json::from_str(&body(response).await).unwrap()
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
    async fn a_described_error_has_the_status_of_its_code_and_no_challenge() {
        let response =
            ApiError::new(OAuthErrorCode::InvalidGrant, "Invalid code verifier.").into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get(header::WWW_AUTHENTICATE).is_none());
        assert_eq!(
            json_body(response).await,
            json!({
                "error": "invalid_grant",
                "error_description": "Invalid code verifier.",
            })
        );
    }

    #[tokio::test]
    async fn a_challenged_error_is_repeated_in_the_header() {
        let response = ApiError::challenged(
            OAuthErrorCode::InvalidToken,
            "The access token is invalid or expired.",
            Challenge::bearer(),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            challenge(&response),
            r#"Bearer error="invalid_token", error_description="The access token is invalid or expired.""#
        );
        assert_eq!(
            json_body(response).await,
            json!({
                "error": "invalid_token",
                "error_description": "The access token is invalid or expired.",
            })
        );
    }

    #[tokio::test]
    async fn a_challenge_can_name_the_required_scope() {
        let response = ApiError::challenged(
            OAuthErrorCode::InsufficientScope,
            "The access token was not granted the openid scope.",
            Challenge::bearer().with_scope("openid"),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            challenge(&response),
            r#"Bearer error="insufficient_scope", error_description="The access token was not granted the openid scope.", scope="openid""#
        );
    }

    #[tokio::test]
    async fn unauthenticated_only_names_the_scheme_without_a_body() {
        let response = ApiError::Unauthenticated(Scheme::Bearer).into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(challenge(&response), "Bearer");
        assert_eq!(body(response).await, "");
    }

    #[tokio::test]
    async fn a_challenge_that_is_no_valid_header_becomes_a_server_error() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        let response = ApiError::challenged(
            OAuthErrorCode::InvalidToken,
            "new\nline",
            Challenge::bearer(),
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(header::WWW_AUTHENTICATE).is_none());
        assert!(capture.contents().contains("not a valid header value"));
    }

    #[tokio::test]
    async fn logs_a_server_error_without_showing_it_or_a_challenge() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        let response = ApiError::from(InternalError::Invariant("postgres is down")).into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(header::WWW_AUTHENTICATE).is_none());
        assert_eq!(
            json_body(response).await,
            json!({
                "error": "server_error",
                "error_description": SERVER_ERROR_DESCRIPTION,
            })
        );
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("postgres is down"), "{log}");
    }
}
