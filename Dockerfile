FROM rust:1.78-bullseye as builder
WORKDIR /usr/src/app

# Install native deps needed for some crates (pkg-config, OpenSSL dev headers)
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates clang libclang-dev llvm-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
RUN mkdir -p src && echo "fn main() { println!(\"dummy\"); }" > src/main.rs

# Build dependencies only to cache
RUN cargo build --release || true

# Copy source and build the real binary
COPY . ./
RUN cargo build --release


FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /usr/src/app/target/release/solana-copy-bot /usr/local/bin/solana-copy-bot

# Copy runtime config files (overridden by bind-mounts in compose)
COPY config.toml /app/config.toml

EXPOSE 8080

# Create non-root user
RUN useradd -m -u 1000 appuser || true
USER 1000

WORKDIR /app
ENTRYPOINT ["/usr/local/bin/solana-copy-bot"]
