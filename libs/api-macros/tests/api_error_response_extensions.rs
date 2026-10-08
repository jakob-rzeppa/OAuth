//! `#[code(..)]`, `#[server_error]`, `#[into_response]` and `render = ..`.

use api_macros::ApiErrorResponse;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

/// A protocol's error codes, with the status each one maps to.
pub enum Code {
    InvalidRequest,
    InvalidClient,
}

impl Code {
    pub fn status(&self) -> StatusCode {
        match self {
            Code::InvalidRequest => StatusCode::BAD_REQUEST,
            Code::InvalidClient => StatusCode::UNAUTHORIZED,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Code::InvalidRequest => "invalid_request",
            Code::InvalidClient => "invalid_client",
        }
    }
}

#[derive(Debug)]
pub struct Source(&'static str);

#[ApiErrorResponse]
pub enum TestErrorResponse {
    #[code(Code::InvalidRequest)]
    #[description("The field {0} is invalid.")]
    InvalidField(String),

    #[code(Code::InvalidClient)]
    #[description("The client was not found.")]
    ClientNotFound,

    #[server_error]
    ServerError(Source),

    #[into_response]
    LoginRequired(Redirect),
}

fn render_text(status: StatusCode, error: &str, description: &str) -> Response {
    (status, format!("{error}: {description}")).into_response()
}

#[ApiErrorResponse(render = render_text, headers(header::CACHE_CONTROL => "no-store"))]
pub enum RenderedErrorResponse {
    #[code(Code::InvalidRequest)]
    #[description("Bad.")]
    Bad,
}

async fn body_of(response: Response) -> (StatusCode, String) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn code_sets_the_status_and_error_code() {
    let (status, body) = body_of(TestErrorResponse::ClientNotFound.into_response()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body,
        r#"{"error":"invalid_client","error_description":"The client was not found."}"#
    );
}

#[tokio::test]
async fn code_variants_still_interpolate_the_description() {
    let response = TestErrorResponse::InvalidField("email".to_string()).into_response();

    let (status, body) = body_of(response).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        r#"{"error":"invalid_request","error_description":"The field email is invalid."}"#
    );
}

#[tokio::test]
async fn server_error_hides_its_source() {
    let response = TestErrorResponse::ServerError(Source("connection refused")).into_response();

    let (status, body) = body_of(response).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        body,
        r#"{"error":"server_error","error_description":"An unexpected error occurred while processing the request."}"#
    );
}

#[test]
fn server_error_converts_from_its_source() {
    fn fails() -> Result<(), TestErrorResponse> {
        Err(Source("connection refused"))?
    }

    assert!(matches!(
        fails(),
        Err(TestErrorResponse::ServerError(Source("connection refused")))
    ));
}

#[test]
fn into_response_returns_the_inner_response() {
    let response = TestErrorResponse::LoginRequired(Redirect::to("/login")).into_response();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/login");
}

#[tokio::test]
async fn render_replaces_the_json_body_and_keeps_the_headers() {
    let response = RenderedErrorResponse::Bad.into_response();

    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let (status, body) = body_of(response).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, "invalid_request: Bad.");
}
