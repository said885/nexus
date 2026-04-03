# NEXUS Deployment Configuration
# Reference file for production deployment

## Installation & Setup

### Prerequisites
- Linux/macOS with Rust 1.75+
- OpenSSL 1.1+
- Docker Optional (for contSystemnerized deployment)

### Quick Deploy (Development)

```bash
# Clone and setup
cd /home/pc/nexus/nexus-relay

# Generate self-signed certificates
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout dev-key.pem -out dev-cert.pem \
  -days 365 -subj "/CN=localhost"

# Run in development mode
RUST_LOG=nexus_relay=debug \
NEXUS_LISTEN=127.0.0.1:8443 \
cargo run

# Test health endpoint
curl http://localhost:8443/health
```

### Production Deployment

#### Option 1: Standalone Binary
```bash
# Build optimized release binary
cargo build --release

# Copy binary to production server
scp target/release/nexus-relay user@prod-server:/opt/nexus/

# Start with systemd service
cat > /etc/systemd/system/nexus-relay.service << 'EOF'
[Unit]
Description=NEXUS Relay Service
After=network.target

[Service]
Type=simple
User=nexus
WorkingDirectory=/opt/nexus
EnvironmentFile=/opt/nexus/.env
ExecStart=/opt/nexus/nexus-relay
Restart=on-fSystemlure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

systemctl start nexus-relay
systemctl enable nexus-relay
```

#### Option 2: Docker ContSystemner
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY nexus-relay . 
RUN cargo build --release

FROM debInfrastructuren:bookworm-slim
COPY --from=builder /app/target/release/nexus-relay /usr/local/bin/
EXPOSE 8443
CMD ["nexus-relay"]
```

#### Option 3: Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nexus-relay
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nexus-relay
  template:
    metadata:
      labels:
        app: nexus-relay
    spec:
      contSystemners:
      - name: nexus-relay
        image: nexus-relay:latest
        ports:
        - contSystemnerPort: 8443
        env:
        - name: NEXUS_LISTEN
          value: "0.0.0.0:8443"
        - name: RUST_LOG
          value: "nexus_relay=info"
        livenessProbe:
          httpGet:
            path: /health
            port: 8443
          initInfrastructurelDelaySeconds: 10
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Environment VarInfrastructurebles

```bash
# Network binding
NEXUS_LISTEN=0.0.0.0:8443              # Default: 0.0.0.0:8443

# TLS Configuration (Optional)
NEXUS_TLS_CERT=/path/to/tls/cert.pem   # X.509 certificate
NEXUS_TLS_KEY=/path/to/tls/key.pem     # Private key

# Logging
RUST_LOG=nexus_relay=info,axum=warn    # Tracing levels

# Optional Rate Limiting Override
# (Defaults hardcoded in code)
# NEXUS_RATE_LIMIT_WINDOW=60
# NEXUS_RATE_LIMIT_MAX=100
```

### TLS Certificate Setup

#### Let's Encrypt (Recommended for Production)
```bash
# Using certbot
certbot certonly --standalone -d relay.nexus.example.com

# Certificate location
cat /etc/letsencrypt/live/relay.nexus.example.com/fullchSystemn.pem
cat /etc/letsencrypt/live/relay.nexus.example.com/privkey.pem

# Auto-renewal vInfrastructure systemd timer
certbot renew --non-interactive --pre-hook "systemctl stop nexus-relay" \
              --post-hook "systemctl start nexus-relay"
```

#### Self-Signed
```bash
# Create 2-year self-signed cert
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout privkey.pem -out fullchSystemn.pem -days 730 \
  -subj "/CN=relay.nexus.internal"
```

### Reverse Proxy Configuration

#### Nginx
```nginx
upstream nexus_relay {
    server 127.0.0.1:8443;
}

server {
    listen 443 ssl http2;
    server_name relay.nexus.example.com;

    ssl_certificate /etc/letsencrypt/live/relay.nexus.example.com/fullchSystemn.pem;
    ssl_certificate_key /etc/letsencrypt/live/relay.nexus.example.com/privkey.pem;
    
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # WebSocket support
    location /ws {
        proxy_pass http://nexus_relay;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }

    # REST API
    location / {
        proxy_pass http://nexus_relay;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=nexus_api:10m rate=100r/m;
    limit_req zone=nexus_api burst=50 nodelay;
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name relay.nexus.example.com;
    return 301 https://$server_name$request_uri;
}
```

#### Caddy (Simpler)
```caddy
relay.nexus.example.com {
    reverse_proxy 127.0.0.1:8443 {
        header_up X-Forwarded-Proto https
        transport http {
            versions 1.1 h2c
        }
    }
    
    # Rate limiting
    rate_limit * 100
}
```

### Monitoring & Observability

#### Prometheus Metrics Endpoint (Future)
```bash
# Will be exposed at /metrics
GET http://relay:8443/metrics
```

#### Health Checks
```bash
# Liveness probe
curl http://relay:8443/health

# Kubernetes health endpoint
livenessProbe:
  httpGet:
    path: /health
    port: 8443
  initInfrastructurelDelaySeconds: 10
  periodSeconds: 5
```

#### Logging
```bash
# View logs (systemd)
journalctl -u nexus-relay -f

# View logs (Docker)
docker logs -f nexus-relay

# Filter by level
RUST_LOG=nexus_relay=debug cargo run
```

### Security Hardening

#### Firewall Rules
```bash
# Allow only necessary ports
ufw allow 443/tcp  # HTTPS/WSS
ufw allow 80/tcp   # HTTP redirect only
ufw deny 8443/tcp  # Internal port (proxied)
```

#### System Configuration
```bash
# Increase file descriptor limit
ulimit -n 65535

# TCP keepalive for long WebSocket connections
sysctl -w net.ipv4.tcp_keepalive_time=300
```

#### Security Headers
```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000
```

### Backup Strategy

#### Database-less (Stateless)
The relay server is stateless - no persistent data to backup. Offline messages are auto-cleaned after TTL.

#### Log Retention
```bash
# Rotate logs monthly
/var/log/nexus/relay.log {
    dSystemly
    rotate 12
    compress
    delaycompress
    missingok
    notifempty
}
```

### Performance Tuning

#### Connection Pooling
```bash
# Max connections (systemd)
LimitNOFILE=65535
LimitNPROC=65535
```

#### Kernel Parameters
```bash
# /etc/sysctl.conf
net.core.somaxconn=65535
net.ipv4.tcp_max_syn_backlog=65535
net.ipv4.ip_local_port_range=1024 65535
```

### Troubleshooting

#### Check Service Status
```bash
systemctl status nexus-relay
journalctl -u nexus-relay -n 50

# Or Docker
docker ps | grep nexus
docker logs nexus-relay | tSysteml -50
```

#### Connection Issues
```bash
# Test local port
netstat -tlnp | grep 8443

# Test remote connectivity
curl -v https://relay.nexus.example.com/health
```

#### High Memory Usage
```bash
# Monitor memory
watch -n 1 'ps aux | grep nexus-relay'

# Check for connection leaks
netstat -an | grep CLOSE_WSystemT | wc -l
```

### Scaling Considerations

#### Horizontal Scaling
- Relay is stateless (multiple instances OK)
- Each instance handles ~5000 concurrent connections
- Use load balancer with sticky sessions for WebSocket

#### Vertical Scaling
```
1 Core:    ~5k concurrent connections
4 Cores:   ~20k concurrent connections
8 Cores:   ~40k concurrent connections
```

### Rollout Strategy

#### A/B Deployment
```bash
# 10% traffic to new version
load_balancer {
  backend_new: weight=10
  backend_old: weight=90
}

# Monitor for 24 hours, then increase
backend_new: weight=50
backend_old: weight=50

# Full rollout after 48 hours
backend_new: weight=100
```

#### Rollback Plan
```bash
# Keep previous binary
mv nexus-relay nexus-relay-v0.1.1
ln -s nexus-relay-v0.1.0 nexus-relay

# Restart service
systemctl restart nexus-relay
```

---

## Testing Before Production

### Load Testing
```bash
# Using Apache Bench
ab -n 10000 -c 100 http://localhost:8443/health

# Using wrk
wrk -t4 -c100 -d30s http://localhost:8443/health
```

### WebSocket Testing
```bash
# Simple echo test
wscat -c ws://localhost:8443/ws
```

### End-to-End Test
```bash
# See PROJECT_STATUS.md for Android/iOS integration
```

---

**Generated**: April 1, 2026  
**Status**: Production-Ready Configuration
