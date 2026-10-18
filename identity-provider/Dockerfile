# ---------------------------------------------------------------------------
# migrator: one-shot image that applies the migrations and exits.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS migrator

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install sqlx-cli --version ^0.9 --locked --no-default-features --features postgres,rustls

WORKDIR /app
COPY migrations ./migrations

# Reads DATABASE_URL from the environment.
ENTRYPOINT ["sqlx"]
CMD ["migrate", "run", "--source", "migrations"]

# ---------------------------------------------------------------------------
# dev: rebuilds and restarts the server on every source change.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS dev

# git for the api-macros git dependency, curl for the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-watch --locked

WORKDIR /app

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock sqlx.toml ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY seed ./seed
COPY src ./src

# The target volume outlives the image, and COPY and `watch` sync keep the host
# mtimes. Cargo would treat sources older than the cached binary as unchanged and
# skip the rebuild, so touch them on every start.
CMD ["sh", "-c", "find src -name '*.rs' -exec touch {} + && exec cargo watch -x run"]

# ---------------------------------------------------------------------------
# builder: compiles the identity-provider binary.
#
# The sqlx query macros are checked offline against the metadata in .sqlx, since no database is reachable during the build.
# Regenerate it with `cargo sqlx prepare` after changing a query or migration,
# see DEV.md.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS builder

# git is needed for the api-macros git dependency, curl for utoipa-swagger-ui's
# build script.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY .cargo ./.cargo
COPY Cargo.toml Cargo.lock sqlx.toml ./
COPY .sqlx ./.sqlx
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/identity-provider /usr/local/bin/identity-provider

# ---------------------------------------------------------------------------
# runtime: the server itself. Does not run migrations.
# ---------------------------------------------------------------------------
FROM debian:trixie-slim AS runtime

# curl is used by the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --no-create-home identity-provider

WORKDIR /app
COPY --from=builder /usr/local/bin/identity-provider ./identity-provider
# Read at runtime via the relative path seed/roles.yaml.
COPY seed ./seed

USER identity-provider
ENTRYPOINT ["./identity-provider"]
