use crate::{
    api::roles::query::response::QueryRolesResponse, persistence::roles::find_all::find_all_roles,
};

pub mod response;

#[utoipa::path(
    get,
    path = "/v1/roles",
    tag = "roles",
    responses(
        (status = 200, description = "Query roles", body = QueryRolesResponse),
    )
)]
#[axum::debug_handler]
pub async fn query_roles_endpoint() -> QueryRolesResponse {
    let roles = find_all_roles();

    QueryRolesResponse {
        roles: roles.iter().map(|user| user.into()).collect(),
    }
}
