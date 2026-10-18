use crate::{
    api::{
        openapi::ErrorBody,
        users::authenticate::{
            error_response::AuthenticateUserErrorResponse, request::AuthenticateUserRequest,
            response::AuthenticateUserResponse,
        },
    },
    application::users::authenticate::{AuthenticateUserError, authenticate_user},
};

pub mod error_response;
pub mod request;
pub mod response;

#[utoipa::path(
    post,
    path = "/v1/users/authenticate",
    tag = "users::authenticate",
    request_body = AuthenticateUserRequest,
    responses(
        (status = OK, description = "The user was authenticated.", body = AuthenticateUserResponse),
        (status = UNAUTHORIZED, description = "The provided credentials were invalid.", body = ErrorBody,
            example = json!({"error": "invalid_credentials", "error_description": "The provided credentials are invalid."}),
        ),
        (status = NOT_FOUND, description = "No user with this user name exists.", body = ErrorBody,
            example = json!({"error": "user_not_found", "error_description": "No user with this user name exists."}),
        ),
        (status = INTERNAL_SERVER_ERROR, description = "An internal server error occurred.", body = ErrorBody,
            example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."}),
        ),
        (status = BAD_REQUEST, description = "The request body was invalid.", body = ErrorBody,
            example = json!({"error": "invalid_request", "error_description": "Invalid request body."}),
        )
    ),
)]
#[axum::debug_handler]
pub async fn authenticate_user_endpoint(
    AuthenticateUserRequest {
        user_name,
        password,
    }: AuthenticateUserRequest,
) -> Result<AuthenticateUserResponse, AuthenticateUserErrorResponse> {
    let user_projection = authenticate_user(&user_name, &password)
        .await
        .map_err(|e| match e {
            AuthenticateUserError::InternalError => {
                AuthenticateUserErrorResponse::InternalServerError
            }
            AuthenticateUserError::UserNotFound => AuthenticateUserErrorResponse::UserNotFound,
            AuthenticateUserError::InvalidCredentials => {
                AuthenticateUserErrorResponse::Unauthorized
            }
        })?;

    Ok(AuthenticateUserResponse {
        data: user_projection,
    })
}
