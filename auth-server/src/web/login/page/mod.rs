mod response;

use crate::{
    config::login_session_ttl,
    domain::entity::login_session::LoginSession,
    persistence::login_session::save::{SaveLoginSessionError, save_login_session},
    util::{
        csrf::create_csrf_token,
        session::{create_session_token, hash_session_token},
    },
    web::login::{
        error_response::LoginErrorResponse,
        page::response::LoginPageResponse,
        return_to::{ReturnToQuery, is_local_path},
    },
};

/// Displays the login form.
///
/// Starts a login session: the CSRF token is stored server side under the hash of a fresh session
/// token, which is handed to the browser as a cookie. The CSRF token is also embedded in the form,
/// so the submit endpoint can check that the form was served together with that cookie.
pub async fn login_page_endpoint(
    ReturnToQuery { return_to }: ReturnToQuery,
) -> Result<LoginPageResponse, LoginErrorResponse> {
    if !is_local_path(&return_to) {
        return Err(LoginErrorResponse::InvalidReturnTo);
    }

    login_page(return_to, None, None).await
}

/// Starts a login session and builds the login form for it.
/// `user_name` pre-fills the user name field, e.g. after a failed login attempt.
pub(super) async fn login_page(
    return_to: String,
    user_name: Option<String>,
    error_message: Option<String>,
) -> Result<LoginPageResponse, LoginErrorResponse> {
    let session_token = create_session_token();
    let csrf_token = create_csrf_token();
    let session_ttl_seconds = login_session_ttl();

    save_login_session(
        &hash_session_token(&session_token),
        LoginSession::new(csrf_token.clone()),
        session_ttl_seconds,
    )
    .await
    .map_err(|error| match error {
        SaveLoginSessionError::DatabaseError | SaveLoginSessionError::SerializationError => {
            LoginErrorResponse::DatabaseError
        }
    })?;

    Ok(LoginPageResponse {
        csrf_token,
        return_to,
        session_token,
        session_ttl_seconds,
        user_name,
        error_message,
    })
}
