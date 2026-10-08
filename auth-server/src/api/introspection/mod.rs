use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::Digest;

use crate::{
    api::introspection::{
        error_response::IntrospectionErrorResponse, request::IntrospectionRequest,
        response::IntrospectionResponse,
    },
    config,
    persistence::access_tokens::find_by_token_hash::find_access_token_by_token_hash,
};

mod error_response;
mod request;
mod response;

#[axum::debug_handler]
pub async fn introspection_endpoint(
    IntrospectionRequest { token }: IntrospectionRequest,
) -> Result<IntrospectionResponse, IntrospectionErrorResponse> {
    let token_hash = hash_token(&token);
    let access_token = find_access_token_by_token_hash(&token_hash)
        .await
        .map_err(|_| IntrospectionErrorResponse::DatabaseError)?;

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
        iss: config::iss().to_string(),
    })
}

#[fnmock::mockable]
fn hash_token(token: &str) -> String {
    let hash = sha2::Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}
