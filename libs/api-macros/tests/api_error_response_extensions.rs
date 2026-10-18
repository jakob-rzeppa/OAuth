//! `#[server_error]` and `#[into_response]`.

use api_macros::ApiErrorResponse;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

#[derive(Debug)]
pub struct Source(&'static str);

#[ApiErrorResponse]
pub enum TestErrorResponse {
    #[status_code(StatusCode::NOT_FOUND)]
    #[error("not_found")]
    #[description("The client was not found.")]
    ClientNotFound,

    #[server_error]
    ServerError(Source),

    #[into_response]
    LoginRequired(Redirect),
}

async fn body_of(response: Response) -> (StatusCode, String) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
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
