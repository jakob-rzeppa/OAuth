use axum::{extract::Path, response::Response};

use crate::{
    api::{openapi::ErrorBody, users::delete::error_response::DeleteUserErrorResponse},
    application::users::delete::{DeleteUserError, delete_user},
};

pub mod error_response;

#[utoipa::path(
    delete,
    path = "/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "The user's ID.")),
    responses(
        (status = NO_CONTENT, description = "The user was deleted."),
        (status = BAD_REQUEST, description = "The user ID was not a valid UUID.", body = ErrorBody,
            example = json!({"error": "invalid_user_id", "error_description": "The provided user ID is invalid."}),
        ),
        (status = NOT_FOUND, description = "No user with this ID exists.", body = ErrorBody,
            example = json!({"error": "user_not_found", "error_description": "The user was not found."}),
        ),
        (status = INTERNAL_SERVER_ERROR, description = "An internal server error occurred.", body = ErrorBody,
            example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."}),
        ),
    ),
)]
#[axum::debug_handler]
pub async fn delete_user_endpoint(
    Path(user_id): Path<String>,
) -> Result<Response, DeleteUserErrorResponse> {
    let Ok(user_id) = uuid::Uuid::parse_str(&user_id) else {
        return Err(DeleteUserErrorResponse::InvalidUserId);
    };

    delete_user(user_id).await.map_err(|e| match e {
        DeleteUserError::UserNotFound => DeleteUserErrorResponse::UserNotFound,
        DeleteUserError::DatabaseError => DeleteUserErrorResponse::InternalServerError,
    })?;

    Ok(Response::builder()
        .status(204)
        .body(axum::body::Body::empty())
        .unwrap())
}
