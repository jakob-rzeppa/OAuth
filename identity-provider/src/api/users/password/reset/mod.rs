use axum::extract::Path;

use crate::{
    api::{
        openapi::ErrorBody,
        users::password::reset::{
            error_response::ResetPasswordErrorResponse, response::ResetPasswordResponse,
        },
    },
    application::users::reset_password::{ResetUserPasswordError, reset_user_password},
};

pub mod error_response;
pub mod response;

#[utoipa::path(
    delete,
    path = "/v1/users/{user_id}/password",
    tag = "users::password",
    responses(
        (status = 200, description = "Password reset successfully", body = ResetPasswordResponse),
        (status = 400, description = "Invalid user ID", body = ErrorBody,
            example = json!({"error": "invalid_user_id", "error_description": "The provided user ID is invalid."}),
        ),
        (status = 404, description = "User not found", body = ErrorBody,
            example = json!({"error": "user_not_found", "error_description": "The user was not found."}),
        ),
        (status = 500, description = "Internal server error", body = ErrorBody,
            example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."}),
        ),
    ),
)]
#[axum::debug_handler]
pub async fn reset_password_endpoint(
    Path(user_id): Path<String>,
) -> Result<ResetPasswordResponse, ResetPasswordErrorResponse> {
    let Ok(user_id) = uuid::Uuid::parse_str(&user_id) else {
        return Err(ResetPasswordErrorResponse::InvalidUserId);
    };

    let temporary_password = reset_user_password(user_id).await.map_err(|e| match e {
        ResetUserPasswordError::DatabaseError => ResetPasswordErrorResponse::InternalServerError,
        ResetUserPasswordError::UserNotFound => ResetPasswordErrorResponse::UserNotFound,
        ResetUserPasswordError::HashingError => ResetPasswordErrorResponse::InternalServerError,
    })?;

    Ok(ResetPasswordResponse { temporary_password })
}
