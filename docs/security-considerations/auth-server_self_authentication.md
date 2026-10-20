# Self authentication of the auth server

The auth server must be able to access the identity provider to check the credentials of the users.

This is done by self issuing a token to itself, which can be used like any other access token to call the identity provider.

This means an extra request to the introspection endpoint is needed on every request to the identity provider, but this is a tradeoff for not having to store the credentials of the auth server in the identity provider, but might change in the future.
