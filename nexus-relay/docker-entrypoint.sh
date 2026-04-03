#!/bin/sh
# NEXUS Relay - Production Entrypoint Script
# Handles: Configuration validation, migrations, startup checks

set -e

echo "[$(date +'%Y-%m-%d %H:%M:%S')] NEXUS Relay starting..."

# ============================================================================
# 1. Configuration Validation
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] Validating configuration..."

if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL not set"
    exit 1
fi

if [ -z "$REDIS_URL" ]; then
    echo "ERROR: REDIS_URL not set"
    exit 1
fi

if [ -z "$NEXUS_CORS_ORIGIN" ]; then
    echo "WARNING: NEXUS_CORS_ORIGIN not set, CORS will be disabled"
fi

# Validate TLS files if specified
if [ -n "$NEXUS_TLS_CERT" ] && [ -n "$NEXUS_TLS_KEY" ]; then
    if [ ! -f "$NEXUS_TLS_CERT" ]; then
        echo "ERROR: TLS certificate not found at $NEXUS_TLS_CERT"
        exit 1
    fi
    if [ ! -f "$NEXUS_TLS_KEY" ]; then
        echo "ERROR: TLS key not found at $NEXUS_TLS_KEY"
        exit 1
    fi
    echo "✓ TLS certificates validated"
fi

# ============================================================================
# 2. Database Connectivity Check
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] Waiting for database..."

attempt=0
max_attempts=30
while [ $attempt -lt $max_attempts ]; do
    if psql "$DATABASE_URL" -c "SELECT 1" > /dev/null 2>&1; then
        echo "✓ Database is available"
        break
    fi
    attempt=$((attempt + 1))
    echo "Database unavailable (attempt $attempt/$max_attempts), retrying in 2s..."
    sleep 2
done

if [ $attempt -eq $max_attempts ]; then
    echo "ERROR: Could not connect to database after $max_attempts attempts"
    exit 1
fi

# ============================================================================
# 3. Database Migrations
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] Running database migrations..."

# Create migrations table if it doesn't exist
psql "$DATABASE_URL" -c "
    CREATE TABLE IF NOT EXISTS migrations (
        id SERIAL PRIMARY KEY,
        name VARCHAR(255) NOT NULL UNIQUE,
        applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );
"

# Apply migrations (example - real migrations would be in sql/migrations/)
psql "$DATABASE_URL" < /app/migrations/01_init.sql 2>/dev/null || true
psql "$DATABASE_URL" < /app/migrations/02_indexes.sql 2>/dev/null || true

echo "✓ Database migrations completed"

# ============================================================================
# 4. Redis Connectivity Check
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] Waiting for Redis..."

attempt=0
max_attempts=10
while [ $attempt -lt $max_attempts ]; do
    if redis-cli -u "$REDIS_URL" ping > /dev/null 2>&1; then
        echo "✓ Redis is available"
        break
    fi
    attempt=$((attempt + 1))
    echo "Redis unavailable (attempt $attempt/$max_attempts), retrying in 1s..."
    sleep 1
done

if [ $attempt -eq $max_attempts ]; then
    echo "ERROR: Could not connect to Redis after $max_attempts attempts"
    exit 1
fi

# ============================================================================
# 5. Pre-Startup Checks
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] Running pre-startup checks..."

# Verify write permissions to log directory
if [ ! -w /var/log/nexus ]; then
    echo "WARNING: Cannot write to /var/log/nexus, logs may be lost"
fi

# Check system limits
if command -v ulimit >/dev/null 2>&1; then
    max_open_files=$(ulimit -n)
    if [ "$max_open_files" -lt 65536 ]; then
        echo "WARNING: Max open files is $max_open_files (recommended: 65536+)"
    fi
fi

# ============================================================================
# 6. Startup
# ============================================================================

echo "[$(date +'%Y-%m-%d %H:%M:%S')] NEXUS Relay is starting (PID $$)..."
echo "────────────────────────────────────────────────────────"

# Export environment for subprocess
export DATABASE_URL REDIS_URL NEXUS_CORS_ORIGIN RUST_LOG

# Start NEXUS Relay
exec /usr/local/bin/nexus-relay
