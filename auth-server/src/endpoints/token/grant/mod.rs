use crate::domain::access_token::AccessToken;

pub mod authorization_code;

/// What a successful grant issues.
pub struct IssuedTokens {
    /// The access token handed to the client. Only its hash is stored.
    pub access_token: String,
    /// The access token entity stored in the database.
    pub access_token_entity: AccessToken,
    /// Only issued if the `openid` scope was granted.
    pub id_token: Option<String>,
}
