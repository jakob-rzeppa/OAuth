use crate::login::{
    error_response::LoginErrorResponse,
    login_page::{LoginPageResponse, render_login_page},
    return_to::{ReturnToQuery, is_local_path},
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
        tracing::warn!("login page rejected: return_to is not a local path");
        return Err(LoginErrorResponse::InvalidReturnTo);
    }

    render_login_page(return_to, None, None).await
}
