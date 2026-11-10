use uuid::Uuid;

use crate::{
    domain::authorization_code::openid::has_openid_scope,
    persistence::{
        authorization_codes::take::take_authorization_code, clients::find_by_id::find_client_by_id,
    },
    endpoints::token::{
        access_token::generate_access_token,
        error::TokenError,
        grant::IssuedTokens,
        id_token::generate_id_token,
        pkce::{is_valid_code_verifier, verify_code_challenge},
        request::TokenRequest,
    },
};

pub async fn handle_authorization_code_grant(
    request: TokenRequest,
) -> Result<IssuedTokens, TokenError> {
    let Some(client_id) = Uuid::parse_str(&request.client_id).ok() else {
        return Err(TokenError::InvalidClientId);
    };

    if find_client_by_id(&client_id).is_none() {
        tracing::warn!(%client_id, "token request for an unknown client");
        return Err(TokenError::ClientNotFound);
    }

    let Some(code) = &request.code else {
        return Err(TokenError::MissingAuthorizationCode);
    };
    let Some(code_verifier) = &request.code_verifier else {
        return Err(TokenError::MissingCodeVerifier);
    };
    if !is_valid_code_verifier(code_verifier) {
        return Err(TokenError::MalformedCodeVerifier);
    }

    let Some(authorization_code) = take_authorization_code(code).await? else {
        tracing::warn!(%client_id, "token request with an unknown, expired or already used authorization code");
        return Err(TokenError::InvalidAuthorizationCode);
    };

    if authorization_code.client_id() != &client_id {
        tracing::warn!(%client_id, "token request for an authorization code issued to a different client");
        return Err(TokenError::InvalidAuthorizationCode);
    }

    if !verify_code_challenge(
        authorization_code.code_challenge_method(),
        &code_verifier,
        authorization_code.code_challenge(),
    ) {
        tracing::warn!(%client_id, "token request rejected: code_verifier does not match the code_challenge");
        return Err(TokenError::InvalidCodeVerifier);
    }

    let id_token = if has_openid_scope(authorization_code.scope()) {
        Some(generate_id_token(&authorization_code)?)
    } else {
        None
    };

    let (access_token, access_token_entity) = generate_access_token(
        client_id,
        Some(authorization_code.sub().clone()),
        authorization_code.scope(),
    );

    Ok(IssuedTokens {
        access_token,
        access_token_entity,
        id_token,
    })
}
