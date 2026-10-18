# Auth server

The auth server implements OAuth 2.1 (draft). In some cases it deviates from the spec, generally in a direction disallowing less secure behavior. E.g. the server requires clients to use Pushed Authorization Requests (PAR) instead of sending the request parameters in the authorization request.

## Endpoints

- `POST /par` - Pushed Authorization Request
- `GET /authorize` - Authorization Endpoint Page
- `POST /authorize` - Authorization Endpoint Form Submission
- `POST /token` - Token Endpoint

## Pushed Authorization Requests (PAR)

The server implements the PAR endpoint as specified in [RFC 9126](https://datatracker.ietf.org/doc/html/rfc9126). Since PARs are more secure than sending the request parameters in the authorization request, the server requires clients to use PARs for authorization requests.

TODO: Currently the server does not authenticate clients at the PAR endpoint, since confidential clients are not yet supported.

## Error handling

Each endpoint lists its errors in its own enum (`<endpoint>/error_response.rs`) and maps every variant to one of three types in `src/error/`, which handle rendering and logging:

- `ApiError`: a JSON `{error, error_description}` body, optionally repeated in a `WWW-Authenticate` challenge.
- `PageError`: the HTML error page. Used before a client's `redirect_uri` is known to be valid.
- `RedirectError`: a redirect to the client's `redirect_uri` with `error`, `error_description`, `state` and `iss`.

Every error has an `OAuthErrorCode` and a description, except the bare `WWW-Authenticate` challenge for a missing token (RFC 6750 §3.1). Infrastructure failures are an `InternalError`: they are logged once and answered with a generic `server_error`.
