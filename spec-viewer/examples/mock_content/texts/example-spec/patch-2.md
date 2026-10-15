# Example Spec

## Abstract

This spec describes how a client presents a bearer token to a resource
server, and how the resource server answers when the token is missing or
not accepted.

## Terminology

The key words "MUST", "MUST NOT", "SHOULD" and "MAY" are to be interpreted
as described in RFC 2119.

- **Client**: the application making a request on behalf of a user.
- **Resource server**: the server hosting the protected resources.
- **Bearer token**: a token that grants access to whoever holds it.

## Presenting the token

The client MUST send the token in the `Authorization` request header,
using the `Bearer` scheme:

```http
GET /resource HTTP/1.1
Host: server.example.com
Authorization: Bearer mF_9.B5f-4.1JqM
```

The client MUST NOT send the token in the query of the URL.

## Error responses

When a request has no token, or the token is not accepted, the resource
server answers with `401 Unauthorized` and a `WWW-Authenticate` header:

| Error | Meaning |
|---|---|
| `invalid_request` | The request is malformed. |
| `invalid_token` | The token is expired, revoked or malformed. |
| `insufficient_scope` | The token lacks the scope of the resource. |

## Security considerations

Bearer tokens MUST only be sent over TLS. A client SHOULD keep tokens in
memory and MUST NOT write them to logs.
