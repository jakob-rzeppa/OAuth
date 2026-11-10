mod layout;
mod users;

use axum::{
    http::header,
    response::{IntoResponse, Redirect},
    routing,
};

const HTMX: &str = include_str!("../../static/htmx.min.js");

/// /admin
pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/", routing::get(|| async { Redirect::to("/admin/users") }))
        .route("/static/htmx.min.js", routing::get(htmx))
        .nest("/users", users::router())
}

async fn htmx() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        HTMX,
    )
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn serves_htmx_as_javascript() {
        let request = Request::get("/static/htmx.min.js")
            .body(Body::empty())
            .unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert!(response.status().is_success());
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/javascript; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn redirects_the_root_to_the_users_page() {
        let request = Request::get("/").body(Body::empty()).unwrap();

        let response = router().oneshot(request).await.unwrap();

        assert!(response.status().is_redirection());
        assert_eq!(response.headers()[header::LOCATION], "/admin/users");
    }
}
