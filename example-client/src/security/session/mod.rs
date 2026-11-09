use std::{collections::HashMap, sync::Arc};

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::Engine;
use tokio::sync::RwLock;

use crate::security::AuthProgress;

/// The cookie carrying the session id.
const SESSION_COOKIE: &str = "session_id";

pub struct Session {
    progress: AuthProgress,
}

impl Session {
    pub fn progress(&self) -> &AuthProgress {
        &self.progress
    }

    pub fn progress_mut(&mut self) -> &mut AuthProgress {
        &mut self.progress
    }
}

#[derive(Default, Clone)]
pub struct SessionStore(Arc<RwLock<HashMap<String, Arc<RwLock<Session>>>>>);

impl SessionStore {
    pub async fn get(&self, session_id: &str) -> Option<Arc<RwLock<Session>>> {
        let sessions = self.0.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn create(&self) -> (String, Arc<RwLock<Session>>) {
        let session_id = generate_session_id();

        let mut sessions = self.0.write().await;
        let session = Arc::new(RwLock::new(Session {
            progress: AuthProgress::default(),
        }));
        sessions.insert(session_id.clone(), session.clone());
        (session_id, session)
    }

    /// The session the request's cookie names. Without a cookie, or for an unknown id (e.g. after
    /// a restart, the store lives in memory), creates a new session and sets its cookie on the jar.
    pub async fn get_or_create(&self, jar: CookieJar) -> (CookieJar, Arc<RwLock<Session>>) {
        if let Some(cookie) = jar.get(SESSION_COOKIE)
            && let Some(session) = self.get(cookie.value()).await
        {
            return (jar, session);
        }

        let (session_id, session) = self.create().await;
        let cookie = Cookie::build((SESSION_COOKIE, session_id))
            .path("/")
            .http_only(true)
            .secure(true)
            // Lax, not Strict: the browser has to send the cookie on the redirect back from the
            // auth-server, a cross-site top-level navigation.
            .same_site(SameSite::Lax)
            .build();
        (jar.add(cookie), session)
    }
}

fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the OS random number generator failed");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
