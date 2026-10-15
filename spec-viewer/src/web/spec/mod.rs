use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
};

use crate::{
    application::spec_view::ViewSpecError,
    state::AppState,
    web::display::Label,
    web::{
        error::{not_found_page, provider_unavailable_page},
        render,
    },
};

mod content;
mod params;
mod view_model;

use params::ViewParams;
use view_model::SpecViewModel;

/// The text of a spec with its metadata, versions and proposals in a sidebar.
#[derive(Template)]
#[template(path = "spec/page.html")]
struct SpecPage<'a> {
    view: SpecViewModel<'a>,
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
    let Ok((selection, mode)) = params.parse() else {
        return not_found_page();
    };
    let view = match state
        .spec_view_service
        .view_spec(&id, selection, mode)
        .await
    {
        Ok(view) => view,
        Err(ViewSpecError::NotFound) => return not_found_page(),
        Err(ViewSpecError::Provider(error)) => {
            eprintln!("failed to get spec {id}: {error}");
            return provider_unavailable_page();
        }
    };

    let view = SpecViewModel::new(&view);
    render(
        StatusCode::OK,
        &SpecPage {
            view,
            content: content::ContentPartial { view },
            selection: content::SelectionPanel {
                view,
                swap_out_of_band: false,
            },
        },
    )
}
