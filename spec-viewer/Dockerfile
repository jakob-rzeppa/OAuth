# ---------------------------------------------------------------------------
# dev: rebuilds and restarts the server on every source change.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS dev

# curl for the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-watch --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock askama.toml ./
COPY templates ./templates
COPY static ./static
COPY src ./src

# The target volume outlives the image, and COPY and `watch` sync keep the host
# mtimes. Cargo would treat sources older than the cached binary as unchanged and
# skip the rebuild, so touch them on every start.
CMD ["sh", "-c", "find src -name '*.rs' -exec touch {} + && exec cargo watch -x run"]

# ---------------------------------------------------------------------------
# builder: compiles the spec-viewer binary.
# The askama templates and the static files are compiled into the binary.
# ---------------------------------------------------------------------------
FROM rust:1-slim-trixie AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock askama.toml ./
COPY templates ./templates
COPY static ./static
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/spec-viewer /usr/local/bin/spec-viewer

# ---------------------------------------------------------------------------
# runtime: the server itself.
# ---------------------------------------------------------------------------
FROM debian:trixie-slim AS runtime

# curl is used by the compose healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --no-create-home spec-viewer

WORKDIR /app
COPY --from=builder /usr/local/bin/spec-viewer ./spec-viewer

USER spec-viewer
ENTRYPOINT ["./spec-viewer"]
