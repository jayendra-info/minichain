FROM rust:1.75-slim as builder

WORKDIR /app

COPY . .

RUN cargo build --release -p minichain-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/minichain-server /app/minichain-server
COPY --from=builder /app/crates/server/src/entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/entrypoint.sh

EXPOSE 3000

ENTRYPOINT ["/app/entrypoint.sh"]
