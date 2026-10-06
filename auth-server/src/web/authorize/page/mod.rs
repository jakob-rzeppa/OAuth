mod error_response;
mod request;
mod response;

use std::str::FromStr;

use uuid::Uuid;

use crate::{
    persistence::{clients::find_by_id::find_client_by_id, pars::peek::peek_par},
    web::authorize::page::{
        error_response::AuthorizePageErrorResponse, request::AuthorizePageQuery,
        response::AuthorizePageResponse,
    },
};

/// Displays the authorization confirmation page for a previously
/// pushed authorization request (see `POST /par`).
///
/// Only the client's existence and the `client_id` match
/// between the query and the pushed request are checked here;
/// the pushed request itself was already fully validated when it was created
/// and will be validated again when the user submits the confirmation form.
pub async fn authorize_page_endpoint(
    query: AuthorizePageQuery,
) -> Result<AuthorizePageResponse, AuthorizePageErrorResponse> {
    let AuthorizePageQuery {
        client_id,
        request_uri,
    } = query;

    let client_id = client_id.ok_or(AuthorizePageErrorResponse::MissingClientId)?;
    let client_id =
        Uuid::from_str(&client_id).map_err(|_| AuthorizePageErrorResponse::InvalidClientId)?;

    let request_uri = request_uri.ok_or(AuthorizePageErrorResponse::MissingRequestUri)?;

    let par = peek_par(&request_uri)
        .await
        .map_err(|_| AuthorizePageErrorResponse::ServerError)?
        .ok_or_else(|| {
            tracing::warn!(%client_id, "authorize page requested for an unknown, expired or already used request_uri");
            AuthorizePageErrorResponse::RequestNotFound
        })?;

    if par.client_id() != &client_id {
        tracing::warn!(%client_id, "authorize page client_id does not match the pushed request");
        return Err(AuthorizePageErrorResponse::ClientIdMismatch);
    }

    let client = find_client_by_id(&client_id).ok_or_else(|| {
        tracing::warn!(%client_id, "authorize page requested for an unknown client");
        AuthorizePageErrorResponse::ClientNotFound
    })?;

    let scope = par.scope().to_string();

    Ok(AuthorizePageResponse {
        client_name: client.client_name().to_string(),
        scope,
        client_id,
        request_uri,
    })
}
