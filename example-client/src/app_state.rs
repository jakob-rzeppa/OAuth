use crate::security::session::SessionStore;

#[derive(Clone)]
pub struct AppState {
    pub session_store: SessionStore,
}
