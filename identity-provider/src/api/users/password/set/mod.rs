use axum::{extract::Path, response::Response};

use crate::{
    api::{
        openapi::ErrorBody,
        users::password::set::{
            error_response::SetPasswordErrorResponse, request::SetPasswordRequest,
        },
    },
    application::users::set_password::{SetUserPasswordError, set_user_password},
};

pub mod error_response;
pub mod request;

#[utoipa::path(
    put,
    path = "/v1/users/{user_id}/password",
    tag = "users::password",
    request_body = SetPasswordRequest,
    responses(
        (status = 204, description = "Password updated successfully"),
        (status = 400, description = "Invalid request body", body = ErrorBody,
            examples(
                ("invalid_request" = (summary = "Invalid request body", value = json!({"error": "invalid_request", "error_description": "Invalid request body."}))),
                ("invalid_user_id" = (summary = "Invalid user ID", value = json!({"error": "invalid_user_id", "error_description": "The provided user ID is invalid."}))),
            ),
        ),
        (status = 404, description = "User not found", body = ErrorBody,
            example = json!({"error": "user_not_found", "error_description": "The user was not found."}),
        ),
        (status = 401, description = "Invalid credentials", body = ErrorBody,
            example = json!({"error": "invalid_credentials", "error_description": "The provided credentials are invalid."}),
        ),
        (status = 500, description = "Internal server error", body = ErrorBody,
            example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."}),
        ),
    ),
)]
#[axum::debug_handler]
pub async fn set_password_endpoint(
    Path(user_id): Path<String>,
    SetPasswordRequest {
        current_password,
        new_password,
    }: request::SetPasswordRequest,
) -> Result<Response, SetPasswordErrorResponse> {
    let Ok(user_id) = uuid::Uuid::parse_str(&user_id) else {
        return Err(SetPasswordErrorResponse::InvalidUserId);
    };

    set_user_password(user_id, &current_password, &new_password)
        .await
        .map_err(|e| match e {
            SetUserPasswordError::DatabaseError => SetPasswordErrorResponse::InternalServerError,
            SetUserPasswordError::HashingError => SetPasswordErrorResponse::InternalServerError,
            SetUserPasswordError::InvalidCredentials => {
                SetPasswordErrorResponse::InvalidCredentials
            }
            SetUserPasswordError::UserNotFound => SetPasswordErrorResponse::UserNotFound,
        })?;

    Ok(Response::builder()
        .status(204)
        .body(axum::body::Body::empty())
        .unwrap())
}
