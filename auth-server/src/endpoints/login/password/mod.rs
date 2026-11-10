mod request;

use axum::{
    http::header,
    response::{AppendHeaders, IntoResponse, Redirect, Response},
};
use chrono::Utc;

use crate::{
    endpoints::login::{
        error::LoginError,
        password::request::ChangePasswordFormSubmitRequest,
        return_to::{ReturnToQuery, is_local_path},
        template::ChangePasswordPageResponse,
    },
    identity_provider::users::{
        authorize::{AuthorizeUserError, authorize_user},
        set_password::{SetUserPasswordError, set_user_password},
    },
    session::{self, user_session::UserSession},
};

/// Replaces the temporary password the user logged in with, and only then starts the session.
#[axum::debug_handler]
pub async fn change_password_submit_endpoint(
    ReturnToQuery { return_to }: ReturnToQuery,
    request: ChangePasswordFormSubmitRequest,
) -> Result<Response, LoginError> {
    let ChangePasswordFormSubmitRequest {
        user_name,
        current_password,
        new_password,
        new_password_confirmation,
    } = request;

    if !is_local_path(&return_to) {
        tracing::warn!("change password rejected: return_to is not a local path");
        return Err(LoginError::InvalidReturnTo);
    }

    let show_again = |return_to: String, user_name: String, error_message: &str| {
        Ok(ChangePasswordPageResponse {
            return_to,
            user_name,
            error_message: Some(error_message.to_string()),
        }
        .into_response())
    };

    if new_password.is_empty() {
        return show_again(return_to, user_name, "The new password must not be empty.");
    }
    if new_password != new_password_confirmation {
        return show_again(return_to, user_name, "The new passwords do not match.");
    }
    if new_password == current_password {
        return show_again(
            return_to,
            user_name,
            "The new password must differ from the temporary password.",
        );
    }

    let user = match authorize_user(&user_name, &current_password).await {
        Ok(user) => user,
        Err(AuthorizeUserError::Internal(error)) => return Err(error.into()),
        Err(AuthorizeUserError::InvalidCredentials) => {
            tracing::info!("change password failed: invalid credentials");
            return show_again(return_to, user_name, "Invalid temporary password.");
        }
    };

    match set_user_password(user.id, &current_password, &new_password).await {
        Ok(()) => {}
        Err(SetUserPasswordError::Internal(error)) => return Err(error.into()),
        Err(SetUserPasswordError::InvalidCredentials) => {
            tracing::info!(user_id = %user.id, "change password failed: invalid credentials");
            return show_again(return_to, user_name, "Invalid temporary password.");
        }
    }

    let session_cookie = session::issue(&UserSession::new(user.id, Utc::now())).await?;

    tracing::info!(user_id = %user.id, "user changed the temporary password and logged in");

    // 303, so the browser follows up with a GET even though this was a POST.
    Ok((
        AppendHeaders([(header::SET_COOKIE, session_cookie)]),
        Redirect::to(&return_to),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::http::StatusCode;
    use uuid::Uuid;

    use super::*;
    use crate::{
        config::Config,
        identity_provider::users::{
            authorize::{AuthorizedUser, authorize_user_mock},
            set_password::set_user_password_mock,
        },
        logging::testing::LogCapture,
        persistence::user_sessions::save_session_mock,
    };

    fn return_to(return_to: &str) -> ReturnToQuery {
        ReturnToQuery {
            return_to: return_to.to_string(),
        }
    }

    fn form(
        new_password: &str,
        new_password_confirmation: &str,
    ) -> ChangePasswordFormSubmitRequest {
        ChangePasswordFormSubmitRequest {
            user_name: "alice".to_string(),
            current_password: "tmp-secret".to_string(),
            new_password: new_password.to_string(),
            new_password_confirmation: new_password_confirmation.to_string(),
        }
    }

    fn authorized(user_id: Uuid) {
        authorize_user_mock().setup(move |user_name, password| {
            assert_eq!(user_name, "alice");
            assert_eq!(password, "tmp-secret");
            Ok(AuthorizedUser {
                id: user_id,
                has_temporary_password: true,
            })
        });
    }

    fn no_session() {
        save_session_mock().setup(|_, _, _| panic!("no session may be issued"));
    }

    async fn body(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn sets_the_new_password_then_redirects_with_a_session() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let user_id = Uuid::new_v4();
        let saved_session = Arc::new(Mutex::new(None));
        let saved_in_mock = saved_session.clone();
        authorized(user_id);
        set_user_password_mock().setup(move |id, current, new| {
            assert_eq!(id, user_id);
            assert_eq!(current, "tmp-secret");
            assert_eq!(new, "chosen");
            Ok(())
        });
        Config::user_session_ttl_fake().setup(|| 1800);
        save_session_mock().setup(move |_, value, _| {
            *saved_in_mock.lock().unwrap() = Some(value.to_string());
            Ok(())
        });

        let response = change_password_submit_endpoint(
            return_to("/authorize?client_id=1"),
            form("chosen", "chosen"),
        )
        .await
        .ok()
        .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/authorize?client_id=1"
        );
        let cookie = response.headers().get(header::SET_COOKIE).unwrap();
        assert!(
            cookie.to_str().unwrap().starts_with("__Host-user_session="),
            "{cookie:?}"
        );
        let session = saved_session.lock().unwrap().clone().unwrap();
        let session = serde_json::from_str::<UserSession>(&session).unwrap();
        assert_eq!(session.user_id(), user_id);

        let log = capture.contents();
        assert!(
            log.contains("user changed the temporary password and logged in"),
            "{log}"
        );
        assert!(log.contains(&user_id.to_string()), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("tmp-secret"), "{log}");
        assert!(!log.contains("chosen"), "{log}");
    }

    #[tokio::test]
    async fn shows_the_page_again_when_the_new_passwords_do_not_match() {
        no_session();

        let response =
            change_password_submit_endpoint(return_to("/authorize"), form("chosen", "chosne"))
                .await
                .ok()
                .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        let body = body(response).await;
        assert!(body.contains("The new passwords do not match."), "{body}");
        assert!(body.contains("value=\"alice\""), "{body}");
    }

    #[tokio::test]
    async fn shows_the_page_again_when_the_new_password_is_the_temporary_one() {
        no_session();

        let response = change_password_submit_endpoint(
            return_to("/authorize"),
            form("tmp-secret", "tmp-secret"),
        )
        .await
        .ok()
        .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body(response)
                .await
                .contains("The new password must differ from the temporary password.")
        );
    }

    #[tokio::test]
    async fn shows_the_page_again_when_the_new_password_is_empty() {
        no_session();

        let response = change_password_submit_endpoint(return_to("/authorize"), form("", ""))
            .await
            .ok()
            .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body(response)
                .await
                .contains("The new password must not be empty.")
        );
    }

    #[tokio::test]
    async fn shows_the_page_again_when_the_temporary_password_is_wrong() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        authorize_user_mock().setup(|_, _| Err(AuthorizeUserError::InvalidCredentials));
        set_user_password_mock().setup(|_, _, _| panic!("the password may not be set"));
        no_session();

        let response =
            change_password_submit_endpoint(return_to("/authorize"), form("chosen", "chosen"))
                .await
                .ok()
                .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body(response).await.contains("Invalid temporary password."));
        let log = capture.contents();
        assert!(log.contains("change password failed"), "{log}");
        assert!(!log.contains("alice"), "{log}");
    }

    #[tokio::test]
    async fn shows_the_page_again_when_the_identity_provider_rejects_the_change() {
        authorized(Uuid::new_v4());
        set_user_password_mock().setup(|_, _, _| Err(SetUserPasswordError::InvalidCredentials));
        no_session();

        let response =
            change_password_submit_endpoint(return_to("/authorize"), form("chosen", "chosen"))
                .await
                .ok()
                .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body(response).await.contains("Invalid temporary password."));
    }

    #[tokio::test]
    async fn rejects_a_return_to_leaving_this_server() {
        no_session();

        let result = change_password_submit_endpoint(
            return_to("https://evil.example"),
            form("chosen", "chosen"),
        )
        .await;

        assert!(matches!(result, Err(LoginError::InvalidReturnTo)));
    }
}
