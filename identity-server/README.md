# Identity Server

The user source of truth. This service is responsible for managing users and their credentials.

## Endpoints

- `POST /v1/users` - Create a user (returns a temporary password)
- `GET /v1/users` - List all users
  - This request will be changed to use the new QUERY method (with the next axum version) with filtering, sorting and pagination.
- `GET /v1/users/{user_id}` - Get a user
- `PATCH /v1/users/{user_id}` - Update a user's user name, display name or roles
- `DELETE /v1/users/{user_id}` - Delete a user
- `PUT /v1/users/{user_id}/password` - Change a user's password (requires the current password)
- `DELETE /v1/users/{user_id}/password` - Reset a user's password (returns a new temporary password)
- `POST /v1/users/authenticate` - Check a user name and password
- `GET /v1/roles` - List all roles
- `GET /swagger-ui` - Swagger UI, serving the OpenAPI spec at `/api-docs/openapi.json`

## Credentials

The identity server uses a simple username and password authentication. The password is stored as a hash. New users are created with a temporary password, which must be changed on first login (enforced by the auth-server).
