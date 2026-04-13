FROM rust:1.85 AS builder
WORKDIR /app

COPY . .
RUN cargo build --release -p memory-app -p memory-worker

FROM debian:bookworm-slim AS runtime-base
ARG MEAT_MEMORY_VERSION=dev
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN mkdir -p /app/config /app/docs /app/storage/assets
LABEL org.opencontainers.image.source="https://example.invalid/meat-memory"
LABEL org.opencontainers.image.version="${MEAT_MEMORY_VERSION}"
LABEL org.opencontainers.image.description="Meat Memory runtime image"

COPY config /app/config

COPY --from=builder /app/target/release/memory-app /usr/local/bin/memory-app
FROM runtime-base AS app-runtime
COPY --from=builder /app/target/release/memory-app /usr/local/bin/memory-app
EXPOSE 8080
CMD ["memory-app"]

FROM runtime-base AS worker-runtime
COPY --from=builder /app/target/release/memory-worker /usr/local/bin/memory-worker
CMD ["memory-worker"]
