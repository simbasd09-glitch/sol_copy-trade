# Solana Copy Trading Bot for Pump.fun

> **Disclaimer:** This software is provided for educational purposes only. Use of copy trading bots may violate exchange or platform terms of service and could be illegal in your jurisdiction. Always trade responsibly and at your own risk.

## Overview
Rust-based bot that monitors dev wallets on Pump.fun, buys new tokens within specified market cap range, and automatically sells based on configurable take-profit/stop-loss. Designed to achieve first-block inclusion using multi-RPC hedging and Jito bundles.

## Features
- Multi-RPC hedging with latency and error monitoring
- Real-time wallet monitoring via Yellowstone gRPC
- Market cap filter (2k–4k USD) via DexScreener API
- One-trade-per-token enforcement with persistent storage
- Fast buys using Jupiter swaps and Jito bundles
- Configurable buy/sell parameters (runtime reloadable)
- Telegram alerts for every action and low-balance warnings
- Cost tracking ledger with profit/loss calculations
- Docker-ready for 24/7 deployment (Oracle Cloud free tier compatible)
- Health check endpoint and watchdog-friendly container

## Getting Started

### Prerequisites
- Rust toolchain (for building locally)
- Docker & docker-compose for deployment
- Solana wallet with a small balance (~0.4 SOL)
- Telegram bot token (obtain via BotFather)
- Free API keys: Helius, ERPC/Yellowstone gRPC, optional Jito

### Configuration
1. Copy `config.toml` and update your settings, especially `dev_wallets`.
2. Create `.env` from `.env.example` and fill in secret values.
3. Ensure your wallet private key is exported securely (do **not** commit).

### Devnet configuration

This repository includes a `config.devnet.toml` for testing on Solana Devnet. To run the bot with the devnet configuration, set the `CONFIG_PATH` environment variable to the devnet file before running:

```bash
export CONFIG_PATH=config.devnet.toml
cargo run --release
```

On Windows PowerShell use:

```powershell
$env:CONFIG_PATH = 'config.devnet.toml'
cargo run --release
```

The `config.devnet.toml` uses the PublicNode devnet gRPC endpoint at `https://solana-rpc.publicnode.com:10000` and smaller trading parameters so you can test safely.

### Building
```bash
cargo build --release
```

### Running locally
```bash
TELEGRAM_BOT_TOKEN=... WALLET_PRIVATE_KEY_BASE58=... \
  HELIUS_API_KEY=... YELLOWSTONE_GRPC_URL=... \
  cargo run --release
```

### Docker Deployment
```bash
docker build -t solana_copy_bot .
docker-compose up -d
```

- Resources are limited to `3.5` CPUs and `20G` memory in `docker-compose.yml` (tunable).
- Health check at `http://localhost:8080/health` returns 200 when running.

### Runtime Configuration
- Edit `config.toml` and send SIGHUP to the process, or use Telegram `/set` commands to adjust parameters on the fly.
- The Telegram bot listens for `/start` to register your chat; after sending that you will receive alerts.
- Supported commands: `/start`, `/status`, `/settings`, and `/set <key> <value>`.

### Budget Management Tips
- Bot is designed to operate on a ~$40 budget (~0.4 SOL).
- Track all costs (RPC priority fees, Jito tips) in the ledger (`ledger.json`).
- The embedded balance monitor will pause trading automatically below the threshold and notify via Telegram.

### Docker Setup
A simple `Dockerfile` and `docker-compose.yml` are included for deployment on low‑cost VMs like Oracle Cloud.
Build the image locally or on your host with:
```bash
docker build -t solana_copy_bot .
```
Launch the service:
```bash
docker-compose up -d
```

The compose file mounts `config.toml` and `ledger.json` so you can edit them without rebuilding. It also sets resource limits to 3.5 CPUs and 20 GB memory by default (adjustable).

Environment variables must be provided either via an `.env` file or the shell:
- `TELEGRAM_BOT_TOKEN` (required)
- `RPC_CHAINSTACK_URL`, `RPC_PUBLICNODE_URL`, `RPC_HELIUS_URL` (at least one)
- `PHANTOM_PRIVATE_KEY` (base58)
- `GRPC_ENDPOINT` (Yellowstone/ERPC URL)
- `JITO_API_KEY` and `JITO_ENDPOINT` (optional)

Health is exposed at `http://localhost:8080/health` and used by Docker's healthcheck.

### API Keys
- **Helius:** sign up at https://www.helius.dev and get a free API key for RPC and paste into `.env`.
- **ERPC/Yellowstone:** request a free trial or developer account at https://erpc.dev (or similar) for gRPC subscriptions.
- **Telegram:** talk to [@BotFather](https://t.me/BotFather) to create a new bot and obtain the token.
- **Jito:** optional; register to obtain an API key for bundle tips.

### Licence
MIT or Apache-2.0. See `LICENSE`.

### Notes
- Respect rate limits (DexScreener ~300 req/min). Caching is used.
- Bot persists traded mints to avoid duplicates across restarts.
- Use Oracle Cloud free tier with Docker; consider network tuning (increase socket buffers, enable BBR, etc.)

## Code Structure
```
src/
├── main.rs
├── rpc/
│   ├── mod.rs
│   └── latency.rs
├── grpc/
│   ├── mod.rs
│   └── stream_handler.rs
├── trading/
│   ├── mod.rs
│   ├── market_cap.rs
│   ├── swap.rs
│   └── jito.rs
├── config/
│   ├── mod.rs
│   └── settings.rs
├── telegram/
│   ├── mod.rs
│   └── alerts.rs
├── cost_tracker/
│   ├── mod.rs
│   └── ledger.rs
├── utils/
│   ├── mod.rs
│   └── health.rs
└── error.rs
```

Feel free to extend and adapt the bot to your needs. Good luck and trade responsibly! 🛡️
