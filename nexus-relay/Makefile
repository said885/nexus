.PHONY: help build run test clean docker-build docker-up docker-down lint fmt check coverage load-test security-audit

help:
	@echo "NEXUS Relay - Development & Build Commands"
	@echo ""
	@echo "Build & Compilation:"
	@echo "  make build              Build release binary"
	@echo "  make build-debug        Build debug binary"
	@echo "  make clean              Remove build artifacts"
	@echo ""
	@echo "Testing:"
	@echo "  make test               Run all tests"
	@echo "  make test-release       Run tests in release mode"
	@echo "  make coverage           Generate code coverage report"
	@echo "  make fuzz              Run fuzzing tests (short)"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint               Run clippy linter"
	@echo "  make fmt                Format code"
	@echo "  make fmt-check          Check formatting"
	@echo "  make check              Run all checks (fmt, lint, compile)"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build       Build Docker image"
	@echo "  make docker-up          Start Docker Compose stack"
	@echo "  make docker-down        Stop Docker Compose stack"
	@echo "  make docker-logs        View Docker Compose logs"
	@echo ""
	@echo "Performance:"
	@echo "  make load-test          Run load test (requires running server)"
	@echo "  make bench              Run benchmarks"
	@echo ""
	@echo "Security:"
	@echo "  make security-audit     Run security audit"
	@echo "  make cert-generate      Generate self-signed TLS certificates"
	@echo ""
	@echo "Development:"
	@echo "  make run                Run in development mode"
	@echo "  make watch              Run with auto-reload"

# ==============================================================================
# BUILD TARGETS
# ==============================================================================

build:
	@echo "🔨 Building NEXUS Relay (release)..."
	cargo build --release --locked
	@echo "✅ Build complete: ./target/release/nexus-relay"

build-debug:
	@echo "🔨 Building NEXUS Relay (debug)..."
	cargo build
	@echo "✅ Debug build complete: ./target/debug/nexus-relay"

clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf coverage

# ==============================================================================
# TESTING TARGETS
# ==============================================================================

test:
	@echo "🧪 Running tests..."
	cargo test --all-features

test-release:
	@echo "🧪 Running tests (release mode)..."
	cargo test --release --all-features

coverage:
	@echo "📊 Generating code coverage..."
	@command -v cargo-tarpaulin >/dev/null 2>&1 || cargo install cargo-tarpaulin
	cargo tarpaulin --out Html --output-dir coverage
	@echo "✅ Coverage report: coverage/index.html"

fuzz:
	@echo "🔍 Running fuzzing tests (30 seconds)..."
	cargo install cargo-fuzz 2>/dev/null || true
	cd fuzz && timeout 30 cargo fuzz run fuzz_x3dh 2>&1 || true
	cd fuzz && timeout 30 cargo fuzz run fuzz_double_ratchet 2>&1 || true
	@echo "✅ Fuzzing complete"

# ==============================================================================
# CODE QUALITY TARGETS
# ==============================================================================

fmt:
	@echo "📝 Formatting code..."
	cargo fmt --all

fmt-check:
	@echo "📝 Checking code format..."
	cargo fmt --all -- --check

lint:
	@echo "🔍 Running clippy linter..."
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check lint
	@echo "✅ All checks passed"

# ==============================================================================
# DOCKER TARGETS
# ==============================================================================

docker-build:
	@echo "🐳 Building Docker image..."
	docker-compose build

docker-up:
	@echo "🚀 Starting Docker Compose stack..."
	docker-compose up -d
	@echo "✅ Services deployed:"
	@echo "   NEXUS Relay: http://localhost:3000"
	@echo "   Prometheus: http://localhost:9091"
	@echo "   Grafana: http://localhost:3001 (admin/admin)"
	@echo "   pgAdmin: http://localhost:5050 (admin@nexus.local/admin)"

docker-down:
	@echo "⬇️  Stopping Docker Compose stack..."
	docker-compose down

docker-logs:
	@echo "📋 Docker Compose logs (follow mode, Ctrl+C to exit)..."
	docker-compose logs -f

# ==============================================================================
# PERFORMANCE TARGETS
# ==============================================================================

load-test:
	@echo "🔥 Running load test..."
	@command -v k6 >/dev/null 2>&1 || echo "⚠️  k6 not installed. Install: https://k6.io/docs/getting-started/installation/"
	k6 run loadtest/k6-load-test.js --duration 60s --vus 100

bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --all-features

# ==============================================================================
# SECURITY TARGETS
# ==============================================================================

security-audit:
	@echo "🔐 Running security audit..."
	cargo install cargo-audit 2>/dev/null || true
	cargo audit

cert-generate:
	@echo "🔑 Generating self-signed TLS certificates..."
	mkdir -p certs
	openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/certificate.pem -days 365 -nodes \
		-subj "/CN=localhost/O=NEXUS/C=US"
	@echo "✅ Certificates generated in certs/"
	@echo "   ⚠️  These are for development only! Use proper CA-signed certs in production!"

# ==============================================================================
# DEVELOPMENT TARGETS
# ==============================================================================

run:
	@echo "🚀 Running NEXUS Relay (development mode)..."
	cargo run --all-features

watch:
	@echo "👁️  Running with automatic reload..."
	@command -v cargo-watch >/dev/null 2>&1 || cargo install cargo-watch
	cargo watch -x 'run --all-features'
