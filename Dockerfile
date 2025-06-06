# Stage 1: Build Planner
# This stage calculates the dependency graph and produces a recipe file.
# It's small and fast, and only needs to be re-run when Cargo.toml changes.
FROM rust:1.87-slim AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Dependency Cacher
# This stage builds only the dependencies, using the recipe from the planner.
# The result is cached by Docker and re-used as long as the recipe doesn't change.
FROM rust:1.87-slim AS cacher
WORKDIR /app
# Install system dependencies required for building crates like `openssl-sys`
RUN apt-get update -qq && apt-get install -y --no-install-recommends pkg-config libssl-dev
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Final Build
# This stage copies the pre-built dependencies and our application source code,
# then does the final, fast compilation of our own code.
FROM rust:1.87-slim AS builder
WORKDIR /app
# Install system dependencies required for the final link
RUN apt-get update -qq && apt-get install -y --no-install-recommends pkg-config libssl-dev
COPY . .
# Copy over the pre-built dependencies from the cacher stage
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release && strip target/release/shopbot

# Stage 4: Runtime Image
# This is the final, small image that will be deployed.
# It only contains the compiled binary and necessary runtime libraries.
FROM debian:bookworm-slim AS runtime
LABEL org.opencontainers.image.source="https://github.com/your/repo"

# Install runtime dependencies (ca-certificates for HTTPS) and gosu for user switching.
# gosu is a lightweight tool for switching users, a common alternative to sudo.
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl; \
    curl -sSL -o /usr/local/bin/gosu "https://github.com/tianon/gosu/releases/download/1.17/gosu-amd64"; \
    chmod +x /usr/local/bin/gosu; \
    # Clean up build dependencies to keep the image small
    apt-get purge -y --auto-remove curl; \
    rm -rf /var/lib/apt/lists/*;

# Create a non-root user to run the application for better security
RUN useradd --create-home --shell /bin/bash appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/shopbot /usr/local/bin/

# Create the entrypoint script
# This script will fix permissions on the mounted volume before starting the app using gosu.
RUN echo '#!/bin/sh' >> /usr/local/bin/entrypoint.sh && \
    echo 'chown -R appuser:appuser /data' >> /usr/local/bin/entrypoint.sh && \
    echo 'exec gosu appuser "$@"' >> /usr/local/bin/entrypoint.sh && \
    chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["shopbot"]
