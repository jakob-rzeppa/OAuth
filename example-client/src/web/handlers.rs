use askama::Template;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::post,
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    app_state::AppState,
    security::auth::authorization::CallbackQuery,
    web::view::{ACTORS, FlowView},
};

#[derive(Template)]
#[template(path = "flow.html")]
struct FlowPage {
    flow: FlowView,
    actors: [&'static str; 3],
}

#[derive(Template)]
#[template(path = "flow_diagram.html")]
struct FlowDiagram {
    flow: FlowView,
    actors: [&'static str; 3],
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/prepare", post(prepare))
        .route("/par", post(push_par))
        .route("/token", post(exchange_code))
        .route("/reset", post(reset))
}

/// The flow page. Also the redirect URI: a callback from the auth-server is processed, then the
/// browser is sent back to `/` so the code doesn't stay in the address bar or the history.
pub async fn index(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let (jar, session) = state.session_store.get_or_create(jar).await;

    if query.is_callback() {
        session.write().await.progress_mut().handle_callback(query);
        return (jar, Redirect::to("/")).into_response();
    }

    let flow = FlowView::new(session.read().await.progress());
    (jar, render(FlowPage { flow, actors: ACTORS })).into_response()
}

async fn prepare(State(state): State<AppState>, jar: CookieJar) -> Response {
    let (jar, session) = state.session_store.get_or_create(jar).await;
    let mut session = session.write().await;
    session.progress_mut().prepare();
    (jar, diagram(&session)).into_response()
}

async fn push_par(State(state): State<AppState>, jar: CookieJar) -> Response {
    let (jar, session) = state.session_store.get_or_create(jar).await;
    let mut session = session.write().await;
    session.progress_mut().push_par().await;
    (jar, diagram(&session)).into_response()
}

async fn exchange_code(State(state): State<AppState>, jar: CookieJar) -> Response {
    let (jar, session) = state.session_store.get_or_create(jar).await;
    let mut session = session.write().await;
    session.progress_mut().exchange_code().await;
    (jar, diagram(&session)).into_response()
}

async fn reset(State(state): State<AppState>, jar: CookieJar) -> Response {
    let (jar, session) = state.session_store.get_or_create(jar).await;
    let mut session = session.write().await;
    session.progress_mut().reset();
    (jar, diagram(&session)).into_response()
}

fn diagram(session: &crate::security::session::Session) -> Response {
    render(FlowDiagram { flow: FlowView::new(session.progress()), actors: ACTORS })
}

fn render(template: impl Template) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to render template: {err}"),
        )
            .into_response(),
    }
}
