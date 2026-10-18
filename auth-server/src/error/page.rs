use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    error::{InternalError, SERVER_ERROR_DESCRIPTION},
    util::{html::render_error_page, oauth_error::OAuthErrorCode},
};

/// An error answered with the HTML error page, for the endpoints the user's browser visits.
/// Never redirected to the client, e.g. because the client or its `redirect_uri` is not known to
/// be valid (RFC 6749 §4.1.2.1).
#[derive(Debug)]
pub enum PageError {
    /// The page shows the code and the description.
    Described {
        code: OAuthErrorCode,
        description: &'static str,
        /// Overrides the status of the code, e.g. a 403 for a rejected cross-site request.
        status: Option<StatusCode>,
    },
    /// Logged, and answered with a generic `server_error` page.
    Server(InternalError),
}

impl PageError {
    /// A described error with the status of its code.
    pub fn new(code: OAuthErrorCode, description: &'static str) -> Self {
        PageError::Described {
            code,
            description,
            status: None,
        }
    }

    /// A described error with another status than the one of its code.
    pub fn with_status(
        code: OAuthErrorCode,
        description: &'static str,
        status: StatusCode,
    ) -> Self {
        PageError::Described {
            code,
            description,
            status: Some(status),
        }
    }
}

impl From<InternalError> for PageError {
    fn from(error: InternalError) -> Self {
        PageError::Server(error)
    }
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        match self {
            PageError::Described {
                code,
                description,
                status,
            } => render_error_page(status.unwrap_or(code.status()), code.as_str(), description),
            PageError::Server(source) => {
                source.log();
                render_error_page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    OAuthErrorCode::ServerError.as_str(),
                    SERVER_ERROR_DESCRIPTION,
                )
            }
        }
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

    #[tokio::test]
    async fn a_described_error_shows_its_code_with_the_status_of_the_code() {
        let response = PageError::new(
            OAuthErrorCode::InvalidRequestUri,
            "The request_uri parameter has expired.",
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body(response).await;
        assert!(body.contains("<h1>invalid_request_uri</h1>"), "{body}");
        assert!(
            body.contains("The request_uri parameter has expired."),
            "{body}"
        );
    }

    #[tokio::test]
    async fn the_status_can_be_overridden() {
        let response = PageError::with_status(
            OAuthErrorCode::InvalidRequest,
            "This request did not come from this site.",
            StatusCode::FORBIDDEN,
        )
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(body(response).await.contains("<h1>invalid_request</h1>"));
    }

    #[tokio::test]
    async fn logs_a_server_error_without_showing_it() {
        let capture = LogCapture::default();
        let _guard = capture.install();

        let response = PageError::from(InternalError::Invariant("redis is down")).into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = body(response).await;
        assert!(body.contains("<h1>server_error</h1>"), "{body}");
        assert!(!body.contains("redis is down"), "{body}");
        let log = capture.contents();
        assert!(log.contains("ERROR"), "{log}");
        assert!(log.contains("redis is down"), "{log}");
    }
}
