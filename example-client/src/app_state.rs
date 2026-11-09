use crate::security::session::SessionStore;

#[derive(Clone, Default)]
pub struct AppState {
    pub session_store: SessionStore,
}
