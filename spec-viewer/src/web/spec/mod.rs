use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
};

use crate::{
    application::spec_view::{Body, ViewSpecError},
    state::AppState,
    web::display::{CssClass, Label, Sign, TimeDisplay},
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

    render(
        StatusCode::OK,
        &SpecPage {
            view: SpecViewModel::new(&view),
        },
    )
}

/// A draft with a proposal `p` of alice that is selected, shown as the diff of its text.
#[cfg(test)]
fn proposal_diff_view() -> crate::application::spec_view::SpecView {
    use crate::application::{
        diff::diff,
        spec_status::{ProposalState, SpecStatus, Visibility},
        spec_view::{ProposalInfo, Selection, SpecDetail, SpecView, ViewMode},
    };

    SpecView {
        spec: SpecDetail {
            id: "draft".into(),
            title: "Draft".into(),
            abbreviation: "D".into(),
            status: SpecStatus::Draft {
                visibility: Visibility::Public,
            },
            versions: Vec::new(),
            unreleased_patches: Vec::new(),
            proposals: vec![ProposalInfo {
                id: "p".into(),
                proposed_by: "alice".into(),
                created_at: None,
                last_updated_at: None,
                status: ProposalState::ReadyForReview,
                content_uri: String::new(),
            }],
        },
        selection: Some(Selection::Proposal("p".into())),
        mode: ViewMode::Diff,
        body: Body::Diff {
            base: None,
            diff: diff("kept\nold <b>\n", "kept\nnew\n"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_page_with_the_content_and_the_selection_panel_in_place() {
        let view = proposal_diff_view();

        let html = SpecPage {
            view: SpecViewModel::new(&view),
        }
        .render()
        .unwrap();

        assert!(html.contains("<title>Draft"), "{html}");
        assert!(html.contains(r#"<article id="content""#), "{html}");
        assert!(html.contains(r#"<div id="selection""#), "{html}");
        assert!(html.contains("<dd>Public</dd>"), "{html}");
        // The selection panel is only swapped out of band in the htmx response.
        assert!(!html.contains("hx-swap-oob"), "{html}");
    }
}
