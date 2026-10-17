use axum::{Router, middleware::map_response, routing::post};

use crate::{
    config::Config,
    introspection::{
        error_response::IntrospectionErrorResponse, request::IntrospectionRequest,
        response::IntrospectionResponse,
    },
    persistence::access_tokens::find_by_token_hash::find_access_token_by_token_hash,
    security::cache_control::no_store,
    util::token::hash_token,
};

mod error_response;
mod request;
mod response;

pub fn router() -> Router {
    Router::new()
        .route("/introspect", post(introspection_endpoint))
        .layer(map_response(no_store))
}

#[axum::debug_handler]
pub async fn introspection_endpoint(
    IntrospectionRequest { token }: IntrospectionRequest,
) -> Result<IntrospectionResponse, IntrospectionErrorResponse> {
    let token_hash = hash_token(&token);
    let access_token = find_access_token_by_token_hash(&token_hash).await?;

    let Some(access_token) = access_token else {
        return Ok(IntrospectionResponse::Inactive);
    };

    if access_token.exp().timestamp() < chrono::Utc::now().timestamp() {
        return Ok(IntrospectionResponse::Inactive);
    }

    Ok(IntrospectionResponse::Active {
        scope: access_token.scope().to_string(),
        client_id: *access_token.client_id(),
        token_type: access_token.token_type().to_string(),
        sub: access_token.sub().cloned(),
        iat: *access_token.iat(),
        exp: *access_token.exp(),
        iss: Config::iss().to_string(),
    })
}

#[cfg(test)]
mod cache_control_tests {
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sends_no_store() {
        // An invalid body is rejected before touching any persistence.
        let request = Request::post("/introspect")
            .body(Body::from("not a form"))
            .unwrap();

        let response = super::router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }
}

#[cfg(test)]
mod body_format_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use fnmock::predicate;
    use tower::ServiceExt;

    use crate::{
        persistence::access_tokens::find_by_token_hash::find_access_token_by_token_hash_mock,
        util::token::hash_token,
    };

    async fn post(content_type: &str, body: &'static str) -> (StatusCode, serde_json::Value) {
        let request = Request::post("/introspect")
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap();

        let response = super::router().oneshot(request).await.unwrap();

        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn reads_a_form_urlencoded_body() {
        let mock = find_access_token_by_token_hash_mock();
        mock.setup(|_| Ok(None));
        mock.expect(predicate::eq(hash_token("the-token"))).once();

        let (status, body) = post("application/x-www-form-urlencoded", "token=the-token").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, serde_json::json!({ "active": false }));
        mock.assert();
    }

    #[tokio::test]
    async fn rejects_a_json_body_as_invalid_request() {
        let (status, body) = post("application/json", r#"{"token":"the-token"}"#).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_request");
    }
}
