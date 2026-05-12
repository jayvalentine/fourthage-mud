FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/fourthage-mud .
COPY data ./data
ENV MUD_DATA_DIR=./data

CMD ["./fourthage-mud"]
