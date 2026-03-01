FROM rust:1.72-bullseye as builder
WORKDIR /app
COPY . /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/solana-copy-bot /usr/local/bin/solana-copy-bot
COPY config.yaml /etc/solana-copy-bot/config.yaml
ENV RUST_LOG=info
CMD ["/usr/local/bin/solana-copy-bot", "--config", "/etc/solana-copy-bot/config.yaml"]
