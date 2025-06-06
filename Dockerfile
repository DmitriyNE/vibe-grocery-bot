# ──────────────────────────────────────────────────────────────
#  Stage 1 — build the binary
# ──────────────────────────────────────────────────────────────
FROM rust:1.77-slim AS builder            # any recent stable tag works
WORKDIR /app

# 1. Save a bit of bandwidth: only install the build deps we need.
RUN apt-get update -qq \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# 2. Cache dependencies: copy Cargo.toml first, do a dummy build.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs
RUN cargo build --release

# 3. Now bring in the real code and re-build; this re-uses the cache above.
COPY src ./src
RUN cargo build --release && strip target/release/shopbot

# ──────────────────────────────────────────────────────────────
#  Stage 2 — runtime image
# ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
LABEL org.opencontainers.image.source="https://github.com/you/shopbot"

# tiny layer so TLS + time-zone stuff works
RUN apt-get update -qq \
 && apt-get install -y --no-install-recommends ca-certificates tzdata \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/shopbot /usr/local/bin/shopbot

# 8080 is arbitrary; Fly will map whatever you expose.
EXPOSE 8080
ENV RUST_LOG=info

CMD ["shopbot"]   # long-poll by default; add args if you use webhooks

