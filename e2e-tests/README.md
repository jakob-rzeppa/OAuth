# e2e-tests

Black-box end-to-end tests that call the running services over the network.

The services are **not** started by the tests. Start them first:

- `identity-provider`: see [`../identity-provider/DEV.md`](../identity-provider/DEV.md)
- `auth-server`: see [`../auth-server/DEV.md`](../auth-server/DEV.md)
- `spec-provider`: gRPC

```sh
cargo test
```

## Writing a test

Put tests in `tests/`. The crate only provides the configuration, a URL formatter per service and an
HTTP client; requests are built with the plain `reqwest` API.

```rust
use e2e_tests::{Config, client};

#[tokio::test]
async fn auth_server_is_healthy() {
    let cfg = Config::load();

    let res = client().get(cfg.auth_url("/health")).send().await.unwrap();

    assert!(res.status().is_success());
}
```

`client()` does not follow redirects, so the `302` that `POST /authorize` answers with, and the
authorization code in its `Location` header, can be inspected.
