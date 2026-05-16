FROM rust:1.88 AS builder
WORKDIR /app

# Copy dependency files first for better layer caching.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy real sources and rebuild.
ARG SQLX_OFFLINE=true
ENV SQLX_OFFLINE=$SQLX_OFFLINE
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/fourthage-mud .
COPY data ./data
ENV MUD_DATA_DIR=./data

CMD ["./fourthage-mud"]
