use axum::{
    Json, Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::signing_key::signing_key;

/// How long clients may cache the key set. Clients refetch on an unknown `kid` regardless.
const CACHE_CONTROL: &str = "public, max-age=3600";

pub fn router() -> Router {
    Router::new().route("/jwks.json", get(jwks_endpoint))
}

async fn jwks_endpoint() -> Response {
    (
        [(
            header::CACHE_CONTROL,
            HeaderValue::from_static(CACHE_CONTROL),
        )],
        Json(signing_key().jwks()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing_key::{TEST_SIGNING_KEY, signing_key_fake};

    #[tokio::test]
    async fn the_key_set_is_cacheable_json() {
        signing_key_fake().setup(|| &TEST_SIGNING_KEY);

        let response = jwks_endpoint().await;

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=3600"
        );
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
