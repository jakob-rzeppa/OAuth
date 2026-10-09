# Auth

A from-scratch implementation of **OAuth 2.1** (and the surrounding RFCs) in Rust, built **for learning**. The goal is to understand how OAuth works.

[OAuth 2.1](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1) is still a draft, so this project follows the current draft and surrounding RFCs like Pushed Authorization Requests in [RFC 9126](https://datatracker.ietf.org/doc/html/rfc9126).

## Surrounding Projects

A OAuth server without some system to use it is not very useful. That's why this repo contains multiple projects.

- [`identity-server`](identity-server/README.md): a user management service that the auth-server uses to check credentials.
- [`auth-server`](auth-server/README.md): the OAuth authorization server.

A [rfc-editor](https://www.rfc-editor.org/) like specification manager, split into three parts:

- `spec-provider`: a resource server that contains the specs and serves them to clients.
- `spec-editor` (TODO): a client for editing the specs, which calls the spec-provider to save them.
- `spec-viewer` (TODO): a client for viewing the specs, which calls the spec-provider to read them.

The services are tested together by [`e2e-tests`](e2e-tests), black-box tests against the running stacks.

## Implemented OAuth concepts

- **Authorization Code flow with PKCE** is the only flow, as in OAuth 2.1 (no implicit or password grants).
- **Pushed authorization requests** are stored in Redis and consumed once.
- **Token introspection**, so resource servers can check if a token is valid.

### Roadmap

- CSRF protection for the authorize endpoint
- [Openid Connect](https://openid.net/specs/openid-connect-core-1_0.html)
- **HTTPS**
- **Client registration and management**
  - Dynamic client registration ([RFC 7591](https://www.rfc-editor.org/info/rfc7591))
  - Client management ([RFC 7592](https://www.rfc-editor.org/info/rfc7592))
- **Token revocation** ([RFC 7009](https://www.rfc-editor.org/info/rfc7009))
- **Refresh tokens** ([RFC 6749](https://www.rfc-editor.org/info/rfc6749))
- **Proof of possession**:
  - DPoP ([RFC 9449](https://www.rfc-editor.org/info/rfc9449))
  - or mTLS ([RFC 8705](https://www.rfc-editor.org/info/rfc8705))
- **Client credentials flow** ([OAuth 2.1](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1#section-4.4))

## License

See [LICENSE](LICENSE).
