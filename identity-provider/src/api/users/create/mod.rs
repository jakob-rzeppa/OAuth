use crate::{
    api::{
        openapi::ErrorBody,
        users::create::{
            error_response::CreateUserErrorResponse, request::CreateUserRequest,
            response::CreateUserResponse,
        },
    },
    application::users::create::{CreateUserApplicationError, create_user},
};

pub mod error_response;
pub mod request;
pub mod response;

#[utoipa::path(
    post,
    path = "/v1/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = CREATED, description = "The user was created.", body = CreateUserResponse),
        (status = BAD_REQUEST, description = "The request body was invalid, or the email was empty.", body = ErrorBody,
            examples(
                ("invalid_request" = (summary = "Invalid request body", value = json!({"error": "invalid_request", "error_description": "Invalid request body."}))),
                ("invalid_user_name" = (summary = "Invalid User Name", value = json!({"error": "invalid_user_name", "error_description": "User name is invalid."}))),
            ),
        ),
        (status = CONFLICT, description = "A user with this user name already exists.", body = ErrorBody,
            example = json!({"error": "user_name_already_exists", "error_description": "User name already exists."}),
        ),
        (status = INTERNAL_SERVER_ERROR, description = "An internal server error occurred.", body = ErrorBody,
            example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."}),
        ),
    ),
)]
#[axum::debug_handler]
pub async fn create_user_endpoint(
    CreateUserRequest {
        user_name,
        role_ids,
    }: CreateUserRequest,
) -> Result<CreateUserResponse, CreateUserErrorResponse> {
    let (id, temporary_password) = create_user(user_name, role_ids)
        .await
        .map_err(|e| match e {
            CreateUserApplicationError::InvalidUserName => CreateUserErrorResponse::InvalidUserName,
            CreateUserApplicationError::UserNameAlreadyExists => {
                CreateUserErrorResponse::UserNameAlreadyExists
            }
            CreateUserApplicationError::DatabaseError => {
                CreateUserErrorResponse::InternalServerError
            }
            CreateUserApplicationError::PasswordHashingError => {
                CreateUserErrorResponse::InternalServerError
            }
        })?;

    Ok(CreateUserResponse {
        id,
        temporary_password,
    })
}
