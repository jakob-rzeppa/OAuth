use chrono::Utc;

use crate::{
    config::Config,
    domain::access_token::{AccessToken, BEARER},
    error::InternalError,
    persistence::{
        access_tokens::save_access_token,
        self_client::get_auth_client,
        self_token::{find_self_token, save_self_token},
    },
    util::token::{hash_token, random_token},
};

#[fnmock::fakeable]
pub async fn issue_self_token() -> Result<String, InternalError> {
    // Use the existing self token if it exists and is still valid
    if let Some(token) = find_self_token().await? {
        return Ok(token);
    }

    let client = get_auth_client();
    let iat = Utc::now();
    let exp = iat + chrono::Duration::seconds(Config::access_token_ttl().into());

    // Generate a unique token
    let token = random_token();
    let token_hash = hash_token(&token);

    let access_token = AccessToken::new(
        token_hash,
        BEARER.to_string(),
        *client.id(),
        None,
        Utc::now(),
        exp,
        client.scope(),
    );

    save_access_token(&access_token).await?;
    save_self_token(&token, Config::access_token_ttl() as u64).await?;

    Ok(token)
}
