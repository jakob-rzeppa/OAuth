# Development

## Local production stack

Pass only `docker-compose.yml`, otherwise the dev override is loaded:

```sh
docker compose -f docker-compose.yml up -d --build
docker compose -f docker-compose.yml down
```

The database is on an internal network and is not reachable from the host.

## Running the dev stack

`docker-compose.dev.yml` switches to the live-reload image and publishes the database
on `127.0.0.1:5432`.

Start everything and live-reload on changes:

```sh
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --watch
```

This streams the server log, including cargo build errors, to the terminal. Plain
`watch` also live-reloads but does not show any log; from another terminal use
`docker compose -f docker-compose.yml -f docker-compose.dev.yml logs -f identity-server`.
Ctrl+C stops the containers.

Changes to `src/`, `seed/`, `.sqlx/`, `Cargo.toml` and `Cargo.lock` are synced into the
container, where cargo-watch rebuilds and restarts the server. Changes to `migrations/`
re-run the migrator. The first start compiles everything and takes a few minutes;
afterwards rebuilds are incremental.

The database is reachable from the host.

## Logging

`LOG_LEVEL` in `.env` sets the minimum level: `debug`, `info`, `warn` or `error` (any case).
It applies to identity-server's own events; libraries (hyper, sqlx) are capped at `warn`. A
level includes every level below it in this table.

| Level | Shows |
|---|---|
| `debug` | every request (method, path, status, latency) |
| `info` | startup, user authenticated, authentication failed, user created / updated / deleted, password changed / reset |
| `warn` | unexpected behaviour: wrong current password on a password change, operations on unknown user ids, invalid or taken user names, role ids missing from the role store |
| `error` | database failures, invalid user rows, password hashing failures, and unrecoverable startup failures (the server exits) |

Logs never contain passwords, password hashes, user names, query strings or request bodies;
users are identified by their id. Read them with `docker compose ... logs -f identity-server`
(see above).

## Query metadata (`.sqlx`)

The `query!` macros are checked offline against the metadata in `.sqlx/`, so neither the image build nor the dev container needs a database. **Commit `.sqlx/`**, and regenerate it whenever a query or a migration changes, otherwise the Docker build fails.

One-time setup of the CLI (needs `sqlx-toml` to read `sqlx.toml`):

```sh
cargo install sqlx-cli --locked --no-default-features --features postgres,rustls,sqlx-toml
```

Regenerate against the dev database (published on `127.0.0.1:5432`, URL taken from
`LOCAL_POSTGRES_URL` in `.env`):

```sh
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d identity-server-db
docker compose -f docker-compose.yml -f docker-compose.dev.yml run --rm identity-server-migrate
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
docker compose run --rm identity-server-migrate
```
