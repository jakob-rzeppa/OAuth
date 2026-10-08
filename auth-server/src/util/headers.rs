use axum::{
    http::{HeaderValue, header},
    response::Response,
};

/// Response layer for HTML pages: never leak the page URL (it carries the request_uri) to other
/// origins.
pub async fn set_referrer_policy(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

/// Response layer for HTML pages: forbid embedding them in frames (clickjacking).
pub async fn prevent_framing(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("frame-ancestors 'none'"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}
