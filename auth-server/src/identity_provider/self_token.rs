use crate::{
    config::Config,
    domain::access_token::generate_access_token,
    error::InternalError,
    persistence::{
        access_tokens::save_access_token,
        self_client::get_auth_client,
        self_token::{find_self_token, save_self_token},
    },
};

#[fnmock::fakeable]
pub async fn issue_self_token() -> Result<String, InternalError> {
    // Use the existing self token if it exists and is still valid
    if let Some(token) = find_self_token().await? {
        return Ok(token);
    }

    let client = get_auth_client();
    let (token, access_token) = generate_access_token(*client.id(), None, &client.scope());

    save_access_token(&access_token).await?;
    save_self_token(&token, Config::access_token_ttl() as u64).await?;

    Ok(token)
}
