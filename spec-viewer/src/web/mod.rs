use askama::Template;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::state::AppState;

mod assets;
mod error;
mod overview;
mod spec;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(assets::router())
        .merge(overview::router())
        .merge(spec::router())
}

/// Renders the template with the given status, or answers 500 if it fails.
fn render(status: StatusCode, template: &impl Template) -> Response {
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(error) => {
            eprintln!("failed to render template: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
