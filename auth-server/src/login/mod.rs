use axum::{Router, middleware::map_response, routing::get};

use crate::{
    login::{page::login_page_endpoint, submit::login_submit_endpoint},
    util::headers::{prevent_framing, set_referrer_policy},
};

mod error_response;
mod login_page;
mod page;
mod return_to;
mod submit;

/// Name of the cookie carrying the login session token.
const LOGIN_SESSION_COOKIE: &str = "login_session";

pub fn router() -> Router {
    Router::new()
        .route(
            "/login",
            get(login_page_endpoint).post(login_submit_endpoint),
        )
        .layer(map_response(set_referrer_policy))
        .layer(map_response(prevent_framing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sets_referrer_policy_and_prevents_framing() {
        // Without return_to the login page responds with an error page,
        // before touching any persistence.
        let request = Request::get("/login").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        let headers = response.headers();
        assert_eq!(headers.get(header::REFERRER_POLICY).unwrap(), "no-referrer");
        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "frame-ancestors 'none'"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    }
}
