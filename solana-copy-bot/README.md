# Solana Copy Trading Bot

This project is a starter implementation for a Solana copy trading bot that watches the Pump.fun program, copies dev trades, manages positions with trailing stop and take-profit, and routes transactions through Helius Sender API or Jito bundles. It includes configuration via `config.yaml` and is intended to run in Docker on a VPS.

Quick start (build locally):

```
cargo build --release
./target/release/solana-copy-bot --config config.yaml
```

Docker:

```
docker build -t solana-copy-bot .
docker run -v $(pwd)/config.yaml:/etc/solana-copy-bot/config.yaml solana-copy-bot
```

Notes:
- Several parts are implemented as clear modules and placeholders for integration with live Helius gRPC and Jito bundling APIs.
- Replace API tokens and RPC URLs in `config.yaml` before running on mainnet.
