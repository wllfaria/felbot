FROM rust:1.87 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/felbot .
COPY --from=builder /app/migrations ./migrations

EXPOSE 8080
CMD ["./felbot"]
