mod authorize;
mod login;

use axum::{
    Router,
    http::{HeaderValue, header},
    middleware::map_response,
    response::Response,
};

pub fn router() -> Router {
    Router::new()
        .merge(authorize::router())
        .merge(login::router())
        .layer(map_response(set_referrer_policy))
        .layer(map_response(prevent_framing))
}

/// Keep page URLs (e.g. the consent page with its `request_uri`) out of the `Referer` header
/// of cross-origin requests, including the redirect to the client (RFC 9700 §4.2.4).
async fn set_referrer_policy(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}

/// Forbid other sites from framing our pages, so the consent page can't be overlaid
/// to trick the user into clicking "Authorize" (clickjacking, RFC 6749 §10.13, RFC 9700 §4.16).
///
/// `frame-ancestors` is the standard mechanism; `X-Frame-Options` covers older browsers.
async fn prevent_framing(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("frame-ancestors 'none'"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn sets_referrer_policy_on_web_responses() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::REFERRER_POLICY).unwrap(),
            "no-referrer"
        );
    }

    #[tokio::test]
    async fn prevents_framing_of_web_responses() {
        // Without query parameters the consent page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/authorize").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .unwrap(),
            "frame-ancestors 'none'"
        );
        assert_eq!(
            response.headers().get(header::X_FRAME_OPTIONS).unwrap(),
            "DENY"
        );
    }
}
