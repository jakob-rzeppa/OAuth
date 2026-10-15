use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
};

use crate::{
    application::spec_view::{SpecView, ViewParams, ViewSpecError},
    state::AppState,
    web::display::Label,
    web::{
        error::{not_found_page, provider_unavailable_page},
        render,
    },
};

mod content;

/// The text of a spec with its metadata, versions and proposals in a sidebar.
#[derive(Template)]
#[template(path = "spec/page.html")]
struct SpecPage<'a> {
    view: &'a SpecView,
    content: content::ContentPartial<'a>,
    selection: content::SelectionPanel<'a>,
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/specs/{id}", get(spec_page))
        .route(content::PATH, get(content::content_partial))
}

async fn spec_page(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ViewParams>,
) -> Response {
    let view = match state.spec_view_service.view_spec(&id, params).await {
        Ok(view) => view,
        Err(ViewSpecError::NotFound) => return not_found_page(),
        Err(ViewSpecError::Provider(error)) => {
            eprintln!("failed to get spec {id}: {error}");
            return provider_unavailable_page();
        }
    };

    render(
        StatusCode::OK,
        &SpecPage {
            view: &view,
            content: content::ContentPartial { view: &view },
            selection: content::SelectionPanel {
                view: &view,
                swap_out_of_band: false,
            },
        },
    )
}
