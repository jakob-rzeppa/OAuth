//! The static files, compiled into the binary like the templates.

use axum::{
    Router,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
    routing::get,
};

const HTMX: &str = include_str!("../../static/htmx.min.js");
const STYLE: &str = include_str!("../../static/style.css");

pub(super) fn router() -> Router {
    Router::new()
        .route(
            "/static/htmx.min.js",
            get(|| async { asset("text/javascript", HTMX) }),
        )
        .route(
            "/static/style.css",
            get(|| async { asset("text/css", STYLE) }),
        )
}

fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, content_type),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        body,
    )
}
