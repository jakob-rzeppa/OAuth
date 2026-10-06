# Development

> **identity-server must be running for auth-server to work.** Start its stack first
> (see `../identity-server/DEV.md`). auth-server reaches it via the port it publishes on
> the host, at `IDENTITY_SERVER_URL` (`http://127.0.0.1:8080`).

## Local production stack

Pass only `docker-compose.yml`, otherwise the dev override is loaded:

```sh
docker compose -f docker-compose.yml up -d --build
docker compose -f docker-compose.yml down
```

The database and redis are on an internal network and are not reachable from the host.
The server is published on `APP_HOST_PORT` (`8081`).

## Running the dev stack

`docker-compose.dev.yml` switches to the live-reload image and publishes the database
on `127.0.0.1:5433` and redis on `127.0.0.1:6379`.

Start everything and live-reload on changes:

```sh
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --watch
```

This streams the server log, including cargo build errors, to the terminal. Plain
`watch` also live-reloads but does not show any log; from another terminal use
`docker compose -f docker-compose.yml -f docker-compose.dev.yml logs -f auth-server`.
Ctrl+C stops the containers.

Changes to `src/`, `seed/`, `templates/`, `.sqlx/`, `Cargo.toml` and `Cargo.lock` are
synced into the container, where cargo-watch rebuilds and restarts the server. Changes
to `migrations/` re-run the migrator. The first start compiles everything and takes a
few minutes; afterwards rebuilds are incremental.

The database and redis are reachable from the host.

## Logging

`LOG_LEVEL` in `.env` sets the minimum level: `debug`, `info`, `warn` or `error` (any case).
It applies to auth-server's own events; libraries (hyper, sqlx, reqwest, redis) are capped at
`warn`. A level includes every level below it in this table.

| Level | Shows |
|---|---|
| `debug` | every request (method, path, status, latency), calls to the identity-server |
| `info` | startup, user logged in, authorization code issued, access token issued, consent denied, failed login |
| `warn` | unexpected behaviour: unknown client, unregistered `redirect_uri`, disallowed scope, unknown or reused `request_uri` / authorization code, invalid CSRF token or PKCE verifier, unsupported `grant_type` |
| `error` | database, redis and identity-server failures, and unrecoverable startup failures (the server exits) |

Logs never contain tokens, codes, `request_uri`, cookies, passwords, user names or query
strings. Read them with `docker compose ... logs -f auth-server` (see above).

## Query metadata (`.sqlx`)

The `query!` macros are checked offline against the metadata in `.sqlx/`
(`.cargo/config.toml` sets `SQLX_OFFLINE=true`), so neither the image build nor the dev
container needs a database. **Commit `.sqlx/`**, and regenerate it whenever a query or
a migration changes, otherwise the Docker build fails.

One-time setup of the CLI (needs `sqlx-toml` to read `sqlx.toml`):

```sh
cargo install sqlx-cli --locked --no-default-features --features postgres,rustls,sqlx-toml
```

Regenerate against the dev database (published on `127.0.0.1:5433`, URL taken from
`LOCAL_POSTGRES_URL` in `.env`):

```sh
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d auth-server-db
docker compose -f docker-compose.yml -f docker-compose.dev.yml run --rm auth-server-migrate
cargo sqlx prepare
```

The database must be migrated to the latest schema first, which the second command
does. To type-check a changed query on the host before preparing, run
`SQLX_OFFLINE=false cargo check`. Verify that `.sqlx/` is up to date with
`cargo sqlx prepare --check`.

## Migrations

Migrations run in their own one-shot container before the server starts. The server
never runs them itself.

```sh
sqlx migrate add <name>
```

After adding or changing a migration, regenerate `.sqlx/` (see above).

Apply pending migrations (also happens automatically on `up` / `watch`):

```sh
docker compose -f docker-compose.yml -f docker-compose.dev.yml run --rm auth-server-migrate
```
