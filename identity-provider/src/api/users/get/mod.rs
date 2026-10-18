use axum::extract::Path;

use crate::{
    api::{
        openapi::ErrorBody,
        users::get::{error_response::GetUserErrorResponse, response::GetUserResponse},
    },
    application::users::get_full::{GetFullUserError, get_full_user_projection},
};

pub mod error_response;
pub mod response;

#[utoipa::path(
    get,
    path = "/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "The user's ID.")),
    responses(
        (status = OK, description = "The user was found.", body = GetUserResponse),
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
pub async fn get_user_endpoint(
    Path(user_id): Path<String>,
) -> Result<GetUserResponse, GetUserErrorResponse> {
    let Ok(user_id) = uuid::Uuid::parse_str(&user_id) else {
        return Err(GetUserErrorResponse::InvalidUserId);
    };

    let user_projection = get_full_user_projection(user_id)
        .await
        .map_err(|e| match e {
            GetFullUserError::UserNotFound => GetUserErrorResponse::UserNotFound,
            GetFullUserError::DatabaseError => GetUserErrorResponse::InternalServerError,
        })?;

    Ok(GetUserResponse {
        data: user_projection,
    })
}
