use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::{
    application::spec_view::{Body, ViewSpecError},
    state::AppState,
    web::display::{CssClass, Label, Sign, TimeDisplay},
    web::error::fragment::{content_not_found_fragment, content_unavailable_fragment},
    web::spec::{SpecViewModel, ViewParams},
};

pub(super) const PATH: &str = "/specs/{id}/content";

/// The text, or the diff, of the selected version or proposal.
#[derive(Template)]
#[template(path = "spec/content.html")]
pub(super) struct ContentPartial<'a> {
    pub(super) view: SpecViewModel<'a>,
}

/// The version selector and proposals in the sidebar, with the details of the selection.
///
/// Sent along with the [`ContentPartial`], replacing the panel out of band.
#[derive(Template)]
#[template(path = "spec/selection.html")]
pub(super) struct SelectionPanel<'a> {
    pub(super) view: SpecViewModel<'a>,
    pub(super) swap_out_of_band: bool,
}

pub(super) async fn content_partial(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ViewParams>,
) -> Response {
    let Ok((selection, mode)) = params.parse() else {
        return content_not_found_fragment();
    };
    let view = match state
        .spec_view_service
        .view_spec(&id, selection, mode)
        .await
    {
        Ok(view) => view,
        Err(ViewSpecError::NotFound) => return content_not_found_fragment(),
        Err(ViewSpecError::Provider(error)) => {
            eprintln!("failed to get spec {id}: {error}");
            return content_unavailable_fragment();
        }
    };

    let view = SpecViewModel::new(&view);
    let rendered = ContentPartial { view }.render().and_then(|content| {
        let selection = SelectionPanel {
            view,
            swap_out_of_band: true,
        }
        .render()?;
        Ok(content + &selection)
    });
    match rendered {
        Ok(html) => {
            let page_url = format!("/specs/{id}?{}", view.query_string());
            ([("HX-Push-Url", page_url)], Html(html)).into_response()
        }
        Err(error) => {
            eprintln!("failed to render template: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        diff::diff,
        spec_status::{ProposalState, SpecStatus, Visibility},
        spec_view::{ProposalInfo, Selection, SpecDetail, SpecView, ViewMode},
    };

    fn proposal_diff_view() -> SpecView {
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

    #[test]
    fn renders_the_diff_escaped() {
        let view = proposal_diff_view();

        let html = ContentPartial {
            view: SpecViewModel::new(&view),
        }
        .render()
        .unwrap();

        assert!(html.contains(r#"<article id="content""#), "{html}");
        assert!(html.contains("Proposal by alice"), "{html}");
        assert!(html.contains("Compared with an empty text"), "{html}");
        assert!(html.contains(r#"<tr class="removed">"#), "{html}");
        assert!(html.contains("old &#60;b&#62;"), "{html}");
        assert!(html.contains(r#"<tr class="added">"#), "{html}");
    }

    #[test]
    fn swaps_the_selection_panel_out_of_band_only_in_the_partial() {
        let view = proposal_diff_view();

        let in_page = SelectionPanel {
            view: SpecViewModel::new(&view),
            swap_out_of_band: false,
        }
        .render()
        .unwrap();
        let in_partial = SelectionPanel {
            view: SpecViewModel::new(&view),
            swap_out_of_band: true,
        }
        .render()
        .unwrap();

        assert!(!in_page.contains("hx-swap-oob"), "{in_page}");
        assert!(in_partial.contains(r#"hx-swap-oob="true""#), "{in_partial}");
        assert!(
            in_partial.contains(r#"aria-current="page""#),
            "{in_partial}"
        );
    }
}
