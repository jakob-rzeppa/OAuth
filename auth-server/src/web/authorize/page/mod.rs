mod error_response;
mod request;
mod response;

use std::str::FromStr;

use uuid::Uuid;

use crate::{
    persistence::{clients::find_by_id::find_client_by_id, pars::peek::peek_par},
    security::{require_session::require_user_session, session::UserSessionToken},
    web::authorize::page::{
        error_response::AuthorizePageErrorResponse, request::AuthorizePageQuery,
        response::AuthorizePageResponse,
    },
};

/// Displays the authorization confirmation page for a previously
/// pushed authorization request (see `POST /par`).
///
/// The user has to be logged in, otherwise they are redirected to the login first.
///
/// Only the client's existence and the `client_id` match
/// between the query and the pushed request are checked here;
/// the pushed request itself was already fully validated when it was created
/// and will be validated again when the user submits the confirmation form.
pub async fn authorize_page_endpoint(
    session_token: Option<UserSessionToken>,
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

    // Only the user themselves may see (and later submit) the request, so log in before showing it.
    require_user_session(session_token, client_id, &request_uri).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::user_session_ttl_fake,
        persistence::user_session::access::{AccessUserSessionError, access_user_session_fake},
        security::require_session::LoginRedirect,
    };
    use axum::response::IntoResponse;

    fn query(client_id: Uuid) -> AuthorizePageQuery {
        AuthorizePageQuery {
            client_id: Some(client_id.to_string()),
            request_uri: Some("urn:authorize:request_uri:test".to_string()),
        }
    }

    fn session_token() -> Option<UserSessionToken> {
        Some(UserSessionToken::new("session-token".to_string()))
    }

    #[tokio::test]
    async fn redirects_to_login_without_a_session_token() {
        let client_id = Uuid::new_v4();

        let result = authorize_page_endpoint(None, query(client_id)).await;

        let Err(AuthorizePageErrorResponse::LoginRequired(LoginRedirect {
            client_id: login_client_id,
            request_uri,
        })) = result
        else {
            panic!("expected a login required error");
        };
        assert_eq!(login_client_id, client_id);
        assert_eq!(request_uri, "urn:authorize:request_uri:test");
    }

    #[tokio::test]
    async fn redirects_to_login_when_the_session_is_expired() {
        user_session_ttl_fake().setup(|| 1800);
        access_user_session_fake().setup(|_, _| Ok(None));

        let result = authorize_page_endpoint(session_token(), query(Uuid::new_v4())).await;

        assert!(matches!(
            result,
            Err(AuthorizePageErrorResponse::LoginRequired(_))
        ));
    }

    #[tokio::test]
    async fn fails_with_server_error_when_access_user_session_fails() {
        user_session_ttl_fake().setup(|| 1800);
        access_user_session_fake().setup(|_, _| Err(AccessUserSessionError::DatabaseError));

        let result = authorize_page_endpoint(session_token(), query(Uuid::new_v4())).await;

        assert!(matches!(
            result,
            Err(AuthorizePageErrorResponse::ServerError)
        ));
    }

    #[tokio::test]
    async fn login_redirect_response_is_a_redirect_to_the_login() {
        let response = AuthorizePageErrorResponse::LoginRequired(LoginRedirect {
            client_id: Uuid::new_v4(),
            request_uri: "urn:authorize:request_uri:test".to_string(),
        })
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        let location = response.headers()[axum::http::header::LOCATION]
            .to_str()
            .unwrap();
        assert!(location.starts_with("/login?return_to="), "{location}");
    }
}
