use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// Renders `template` as an HTML response with `status_code`, or a bare 500 when rendering fails.
pub fn render_html(template: &impl Template, status_code: StatusCode) -> Response {
    match template.render() {
        Ok(html) => (status_code, Html(html)).into_response(),
        Err(error) => {
            tracing::error!(?error, "failed to render template");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Implements `IntoResponse` for templates, rendering them with 200 OK.
macro_rules! html_response {
    ($($template:ty),* $(,)?) => {
        $(impl axum::response::IntoResponse for $template {
            fn into_response(self) -> axum::response::Response {
                crate::admin::layout::html::render_html(&self, axum::http::StatusCode::OK)
            }
        })*
    };
}

pub(crate) use html_response;
