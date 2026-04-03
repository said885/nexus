# Development Mode Best Practices
For local testing and development

## Quick Commands

```bash
# Format code & fix warnings
cargo fmt && cargo clippy --fix

# Run with hot-reload
make watch

# Run tests continuously
cargo watch -x test

# Profile performance
cargo flamegraph

# Copy config template
cp .env.example .env

# Start full stack
make docker-up

# View system health
curl http://localhost:3000/health | jq
```

## Testing Workflows

### Test Message Flow
```bash
# Terminal 1: Start relay
cargo run

# Terminal 2: Test registration
curl -X POST http://localhost:3000/register \
  -H "Content-Type: application/json" \
  -d '{
    "identity_key": "...",
    "signed_prekey": "...",
    "signed_prekey_signature": "...",
    "one_time_prekey": "..."
  }'

# Terminal 3: Fetch prekeys for another user
curl http://localhost:3000/prekeys/{recipient_hash}
```

### Debug Logging
```bash
# Enable debug logs
RUST_LOG=nexus_relay=debug cargo run

# Trace specific module
RUST_LOG=nexus_relay::encryption_manager=trace cargo run
```

## Common Issues

| Issue | Solution |
|-------|----------|
| Port 3000 in use | Change `NEXUS_PORT` in .env or `lsof -i :3000` |
| DB connection refused | Ensure PostgreSQL running: `docker-compose up` |
| Redis not responding | Check Redis: `redis-cli ping` |
| TLS cert missing | Run `make cert-generate` |
