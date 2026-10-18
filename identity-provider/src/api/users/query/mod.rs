use crate::{
    api::{
        openapi::ErrorBody,
        users::query::{error_response::QueryUsersError, response::QueryUsersResponse},
    },
    persistence::users::find_all::{FindAllUsersError, find_all_users},
};

pub mod error_response;
pub mod response;

#[utoipa::path(
    get,
    path = "/v1/users",
    tag = "users",
    responses(
        (status = 200, description = "Query users", body = QueryUsersResponse),
        (status = 500, description = "Internal server error", body = ErrorBody, example = json!({"error": "internal_server_error", "error_description": "An internal server error occurred."})),
    )
)]
#[axum::debug_handler]
pub async fn query_users_endpoint() -> Result<QueryUsersResponse, QueryUsersError> {
    let users = find_all_users().await.map_err(|e| match e {
        FindAllUsersError::DatabaseError => QueryUsersError::InternalServerError,
        FindAllUsersError::InvalidData => QueryUsersError::InternalServerError,
    })?;

    Ok(QueryUsersResponse {
        users: users.iter().map(|user| user.into()).collect(),
    })
}
