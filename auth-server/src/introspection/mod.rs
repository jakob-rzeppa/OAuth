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
            .body(Body::from("not json"))
            .unwrap();

        let response = super::router().oneshot(request).await.unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }
}
