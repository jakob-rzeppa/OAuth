use uuid::Uuid;

use crate::{
    api::token::{
        error_response::TokenErrorResponse, generation::generate_access_token,
        request::TokenRequest, verify_code_challenge,
    },
    domain::entity::access_token::AccessToken,
    persistence::authorization_codes::take::take_authorization_code,
};

pub async fn handle_authorization_code_grant(
    request: TokenRequest,
) -> Result<(String, AccessToken), TokenErrorResponse> {
    let Some(client_id) = Uuid::parse_str(&request.client_id).ok() else {
        return Err(TokenErrorResponse::InvalidClientId);
    };

    let Some(code) = &request.code else {
        return Err(TokenErrorResponse::MissingAuthorizationCode);
    };
    let Some(code_verifier) = &request.code_verifier else {
        return Err(TokenErrorResponse::MissingCodeVerifier);
    };

    let authorization_code = take_authorization_code(&code)
        .await
        .map_err(|_| TokenErrorResponse::DatabaseError);
    let Some(authorization_code) = authorization_code? else {
        tracing::warn!(%client_id, "token request with an unknown, expired or already used authorization code");
        return Err(TokenErrorResponse::InvalidAuthorizationCode);
    };

    if authorization_code.client_id() != &client_id {
        tracing::warn!(%client_id, "token request for an authorization code issued to a different client");
        return Err(TokenErrorResponse::InvalidClientId);
    }

    if !verify_code_challenge(
        authorization_code.code_challenge_method(),
        &code_verifier,
        authorization_code.code_challenge(),
    ) {
        tracing::warn!(%client_id, "token request rejected: code_verifier does not match the code_challenge");
        return Err(TokenErrorResponse::InvalidCodeVerifier);
    }

    Ok(generate_access_token(
        client_id,
        Some(authorization_code.sub().clone()),
        authorization_code.scope(),
    ))
}
