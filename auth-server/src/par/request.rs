use axum::{Json, extract::FromRequest};
use serde::Deserialize;

use crate::par::error_response::ParErrorResponse;

#[derive(Deserialize, FromRequest)]
#[from_request(via(Json), rejection(ParErrorResponse))]
pub struct ParRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub nonce: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_with(nonce: &str) -> String {
        format!(
            r#"{{
                "client_id": "c", "redirect_uri": "r", "response_type": "code",
                "scope": "openid", "state": "s", {nonce}
                "code_challenge": "cc", "code_challenge_method": "S256"
            }}"#
        )
    }

    #[test]
    fn nonce_is_optional() {
        let request: ParRequest = serde_json::from_str(&json_with("")).unwrap();
        assert_eq!(request.nonce, None);
    }

    #[test]
    fn nonce_is_read_when_present() {
        let request: ParRequest = serde_json::from_str(&json_with(r#""nonce": "n","#)).unwrap();
        assert_eq!(request.nonce.as_deref(), Some("n"));
    }
}
