FROM lukemathwalker/cargo-chef:latest-rust-1.83.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
# Build our project dependencies, not our application code
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same, all layers should be cached.
COPY . .

ENV SQLX_OFFLINE true
RUN cargo build --release --bin zero2prod

FROM debian:bullseye-slim as runtime
WORKDIR /app
RUN apt update -y \
    && apt install -y --no-install-recommends openssl ca-certificates \
    # clean up
    && apt autoremove -y && apt clean -y && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configurations configurations
# Assgin APP_ENVIRONMENT="production" to the environment variable. This is used by configuration.rs file.
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]