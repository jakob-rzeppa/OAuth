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
