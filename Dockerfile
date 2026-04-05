FROM rust:1.85 AS builder
WORKDIR /app

COPY . .
RUN cargo build --release -p memory-app -p memory-worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/memory-app /usr/local/bin/memory-app
COPY --from=builder /app/target/release/worker /usr/local/bin/memory-worker
COPY config /app/config

CMD ["memory-app"]
