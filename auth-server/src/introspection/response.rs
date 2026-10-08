use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub enum IntrospectionResponse {
    Active {
        scope: String,
        client_id: Uuid,
        token_type: String,

        sub: Option<Uuid>,

        iat: DateTime<Utc>,
        exp: DateTime<Utc>,
        iss: String,
    },
    Inactive,
}

impl IntoResponse for IntrospectionResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            IntrospectionResponse::Active {
                scope,
                client_id,
                token_type,
                sub,
                iat,
                exp,
                iss,
            } => {
                let response_body = serde_json::json!({
                    "active": true,
                    "scope": scope,
                    "client_id": client_id,
                    "token_type": token_type,
                    "sub": sub,
                    "iat": iat.timestamp(),
                    "exp": exp.timestamp(),
                    "iss": iss,
                });
                axum::Json(response_body).into_response()
            }
            IntrospectionResponse::Inactive => {
                let response_body = serde_json::json!({
                    "active": false
                });
                axum::Json(response_body).into_response()
            }
        }
    }
}
