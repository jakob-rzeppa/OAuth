use std::{collections::HashMap, sync::Arc};

use base64::Engine;
use tokio::sync::RwLock;

use crate::security::AuthProgress;

pub struct Session {
    progress: AuthProgress,
}

#[derive(Default, Clone)]
pub struct SessionStore (Arc<RwLock<HashMap<String, Arc<RwLock<Session>>>>>);

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
}

fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the OS random number generator failed");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
