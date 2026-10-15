# ---------------------------------------------------------------------------
# dev: rebuilds and restarts the server on every source change.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS dev

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-watch --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY spec-api ./spec-api
COPY src ./src

# The target volume outlives the image, and COPY and `watch` sync keep the host
# mtimes. Cargo would treat sources older than the cached binary as unchanged and
# skip the rebuild, so touch them on every start.
CMD ["sh", "-c", "find src spec-api/src -name '*.rs' -exec touch {} + && exec cargo watch -x run"]

# ---------------------------------------------------------------------------
# builder: compiles the spec-provider binary.
# spec-api compiles its proto with a vendored protoc, so none is installed here.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY spec-api ./spec-api
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/spec-provider /usr/local/bin/spec-provider

# ---------------------------------------------------------------------------
# runtime: the server itself.
# ---------------------------------------------------------------------------
FROM debian:trixie-slim AS runtime

RUN useradd --system --no-create-home spec-provider

WORKDIR /app
COPY --from=builder /usr/local/bin/spec-provider ./spec-provider

USER spec-provider
ENTRYPOINT ["./spec-provider"]
