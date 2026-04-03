# NEXUS Relay - Production Dockerfile (v0.2.1)
# Multi-stage build: minimal final image (~45MB)

# ==============================================================================
# Stage 1: Builder
# ==============================================================================
FROM rust:1.75-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    pkg-config \
    libssl-dev \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && rm -rf src

# Copy full source code
COPY src ./src

# Rebuild with actual source (cached dependencies = fast)
RUN cargo build --release --locked

# ==============================================================================
# Stage 2: Runtime
# ==============================================================================
FROM debian:bookworm-slim

WORKDIR /etc/nexus

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/nexus-relay /usr/local/bin/

# Create non-root user for security
RUN useradd -m -u 1000 nexus && chown -R nexus:nexus /etc/nexus

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Security: drop unnecessary capabilities
RUN setcap -r /usr/local/bin/nexus-relay 2>/dev/null || true

# Metadata
LABEL org.opencontainers.image.title="NEXUS Relay"
LABEL org.opencontainers.image.version="0.2.1"
LABEL org.opencontainers.image.description="Post-Quantum End-to-End Encrypted Messaging Relay"
LABEL org.opencontainers.image.source="https://github.com/nexus-messaging/nexus"

# Switch to non-root user
USER nexus

# Expose port
EXPOSE 3000 9090

# Run
ENTRYPOINT ["nexus-relay"]
