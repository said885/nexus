# Installation Guide

This guide covers installation and configuration of NEXUS components for development and production use.

## System Requirements

### Development Environment

- **OS**: Linux, macOS, or Windows (WSL2)
- **Rust**: 1.75 or later (install via [rustup](https://rustup.rs/))
- **PostgreSQL**: 14 or later
- **Redis**: 7.0 or later
- **Git**: 2.0 or later
- **Make**: GNU Make (optional, for convenience)

### Production Environment

- **OS**: Linux (Ubuntu 20.04 LTS, RHEL 8+, or Debian 11+)
- **Kubernetes**: 1.24+ (recommended for high availability)
- **PostgreSQL**: 14+ with streaming replication
- **Redis Cluster**: 7.0+
- **Monitoring**: Prometheus 2.30+, Grafana 8.0+

## Quick Start (Docker)

The fastest way to run NEXUS locally is with Docker Compose:

```bash
# Clone repository
git clone https://github.com/nexus-project/nexus.git
cd nexus

# Start all services
docker-compose up -d

# Verify services are running
docker-compose ps

# View logs
docker-compose logs -f nexus-relay
```

Services will be available at:
- Web UI: http://localhost:3000
- Relay API: http://localhost:8080
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3001

## Local Development Setup

### Install Dependencies

On Ubuntu/Debian:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  postgresql-14 \
  postgresql-contrib-14 \
  redis-server \
  git \
  curl
```

On macOS:

```bash
brew install rust postgresql redis git node
brew services start postgresql
brew services start redis
```

### Initialize Database

Create the NEXUS database and schema:

```bash
# Connect to PostgreSQL
sudo -u postgres psql

# Create database and user
CREATE DATABASE nexus;
CREATE USER nexus WITH PASSWORD 'dev_password';
GRANT ALL PRIVILEGES ON DATABASE nexus TO nexus;

# Exit psql
\q
```

Apply schema migrations:

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run --database-url "postgres://nexus:dev_password@localhost/nexus"
```

Or manually:

```bash
# Apply migration SQL
psql -U nexus -d nexus -f migrations/001_initial_schema.sql
```

### Build Relay Server

```bash
cd nexus-relay

# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Start server
RUST_LOG=debug cargo run --release
```

The relay server will listen on:
- HTTP: `http://localhost:8080`
- WebSocket: `ws://localhost:8080/ws`

### Build Web Client

```bash
cd nexus-web

# Install dependencies
npm install

# Development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

The web client will be available at `http://localhost:5173` in development.

### Build Cryptographic Library

```bash
cd nexus-crypto

# Build library
cargo build --release

# Run tests
cargo test --all

# Generate documentation
cargo doc --open --no-deps
```

### Build Desktop Client (Tauri)

```bash
cd nexus-desktop

# Install Tauri CLI
cargo install tauri-cli

# Development mode
cargo tauri dev

# Build production bundle
cargo tauri build --release
```

## Environment Configuration

### Relay Server Configuration

Create `.env` in `nexus-relay/`:

```bash
# Database connection
DATABASE_URL=postgres://nexus:dev_password@localhost/nexus

# Redis connection
REDIS_URL=redis://localhost:6379

# Server configuration
RUST_LOG=info,nexus_relay=debug
HOST=127.0.0.1
PORT=8080
WORKERS=8

# TLS configuration (production)
TLS_CERT_PATH=/etc/nexus/certs/server.crt
TLS_KEY_PATH=/etc/nexus/certs/server.key
TLS_ENABLED=false

# Security settings
MAX_MESSAGE_SIZE=16384
MAX_GROUP_SIZE=1000
RATE_LIMIT_PER_MINUTE=60
THREAT_DETECTION_ENABLED=true

# Monitoring
PROMETHEUS_ENABLED=true
PROMETHEUS_PORT=9090
```

### Cryptographic Library Configuration

The cryptographic library requires no additional configuration. It can be integrated as:

1. **Rust library** (direct dependency)
2. **C FFI bindings** (Android/iOS)
3. **WASM module** (future: web browsers)

## Running Tests

### Unit Tests

```bash
# All tests
cargo test --all

# Specific package
cargo test -p nexus-relay
cargo test -p nexus-crypto

# With logging
RUST_LOG=debug cargo test -- --nocapture

# Single test
cargo test --lib message_delivery -- --nocapture
```

### Integration Tests

```bash
# Relay integration tests
cd nexus-relay
cargo test --test integration

# Full stack test
cargo test --all --all-targets
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --all --out Html --output-dir coverage

# View report
open coverage/index.html
```

## Verification

After installation, verify everything is working:

```bash
# Check compilation (no warnings should appear)
cargo check --all-targets

# Run clippy linter
cargo clippy --all-targets

# Verify test pass
cargo test --lib

# Build release artifacts
cargo build --all --release
```

Expected output:
```
Finished `release` profile [optimized] target(s) in X.XXs
175 tests passed
0 compiler warnings
```

## Troubleshooting

### PostgreSQL Connection Issues

```bash
# Check if PostgreSQL is running
sudo systemctl status postgresql

# Verify connection
psql -U nexus -d nexus -h localhost

# Check logs
sudo tail -f /var/log/postgresql/postgresql-14-main.log
```

### Redis Connection Issues

```bash
# Check if Redis is running
redis-cli ping

# View Redis logs
redis-cli INFO server
```

### Rust Compilation Errors

```bash
# Update Rust toolchain
rustup update

# Clean build artifacts
cargo clean

# Rebuild
cargo build --release
```

### Missing Dependencies

```bash
# On Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# On macOS
brew install openssl pkg-config
```

## Production Deployment

For production deployment with Kubernetes, see [DEPLOYMENT.md](DEPLOYMENT.md).

Quick Docker image build:

```bash
# Build relay server image
cd nexus-relay
docker build -f Dockerfile.prod -t nexus-relay:latest .

# Push to registry
docker push your-registry/nexus-relay:latest
```

## Getting Help

For assistance with installation:

1. Check [documentation](https://github.com/nexus-project/nexus/tree/main/docs)
2. Open [GitHub Issues](https://github.com/nexus-project/nexus/issues)
3. Visit [GitHub Discussions](https://github.com/nexus-project/nexus/discussions)
4. Read error messages carefully and search for solutions

## Next Steps

After installation:

1. Read the [Architecture Documentation](docs/architecture.md)
2. Review [Security Policies](SECURITY.md)
3. Study [Cryptographic Specification](docs/crypto-spec.md)
4. Start contributing (see [CONTRIBUTING.md](CONTRIBUTING.md))
