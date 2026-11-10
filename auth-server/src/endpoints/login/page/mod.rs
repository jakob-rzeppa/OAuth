use crate::endpoints::login::{
    error::LoginError,
    return_to::{ReturnToQuery, is_local_path},
    template::{LoginPageResponse, render_login_page},
};

/// Displays the login form.
pub async fn login_page_endpoint(
    ReturnToQuery { return_to }: ReturnToQuery,
) -> Result<LoginPageResponse, LoginError> {
    if !is_local_path(&return_to) {
        tracing::warn!("login page rejected: return_to is not a local path");
        return Err(LoginError::InvalidReturnTo);
    }

    render_login_page(return_to, None, None).await
}
