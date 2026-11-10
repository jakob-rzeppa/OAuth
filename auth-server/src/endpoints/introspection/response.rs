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
                let mut response_body = serde_json::json!({
                    "active": true,
                    "scope": scope,
                    "client_id": client_id,
                    "token_type": token_type,
                    "iat": iat.timestamp(),
                    "exp": exp.timestamp(),
                    "iss": iss,
                });
                if let Some(sub) = sub {
                    response_body["sub"] = serde_json::json!(sub);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn active(sub: Option<Uuid>) -> IntrospectionResponse {
        IntrospectionResponse::Active {
            scope: "openid".to_string(),
            client_id: Uuid::new_v4(),
            token_type: "bearer".to_string(),
            sub,
            iat: Utc::now(),
            exp: Utc::now(),
            iss: "https://auth.example.com".to_string(),
        }
    }

    async fn body(response: IntrospectionResponse) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn includes_the_sub_of_a_token_with_a_user() {
        let sub = Uuid::new_v4();

        let body = body(active(Some(sub))).await;

        assert_eq!(body["sub"], sub.to_string());
    }

    #[tokio::test]
    async fn leaves_out_the_sub_of_a_token_without_a_user() {
        let body = body(active(None)).await;

        assert!(body.get("sub").is_none(), "{body}");
        assert_eq!(body["active"], true);
    }
}
