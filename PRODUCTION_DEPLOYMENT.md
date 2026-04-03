# NEXUS Relay - Production Deployment Guide

##  Quick Start (Development)

### 1. Prerequisites
```bash
# Required
- Docker & Docker Compose (v3.9+)
- Rust 1.75+ (for source builds)
- PostgreSQL client tools

# Optional
- k6 (for load testing)
- curl (for API testing)
```

### 2. Setup Development Environment
```bash
# Clone and navigate
cd nexus-relay

# Copy environment configuration
cp .env.example .env

# Generate self-signed TLS certificates (dev only)
make cert-generate

# Start all services with Docker Compose
make docker-up

# View logs
make docker-logs
```

### 3. Verify Deployment
```bash
# Wait for services to be healthy (30-60 seconds)
curl http://localhost:3000/health

# Expected response:
# {
#   "status": "healthy",
#   "version": "0.2.1",
#   "database": "healthy",
#   "redis": "healthy"
# }
```

### 4. Access Service Dashboards
| Service | URL | Credentials |
|---------|-----|-------------|
| **NEXUS Relay** | http://localhost:3000 | No auth |
| **Prometheus** | http://localhost:9091 | No auth |
| **Grafana** | http://localhost:3001 | admin / admin |
| **pgAdmin** | http://localhost:5050 | admin@nexus.local / admin |

---

##  Production Deployment

### 1. Kubernetes Deployment

#### Deploy using Helm Chart
```bash
# Add Nexus Helm repository
helm repo add nexus https://charts.nexus-messaging.io
helm repo update

# Install with custom values
helm install nexus-relay nexus/nexus-relay \
  --namespace nexus \
  --create-namespace \
  -f production-values.yaml
```

#### Manual Kubernetes Deployment
```bash
# Apply manifests in order
kubectl create namespace nexus
kubectl apply -f k8s/postgres-pvc.yaml
kubectl apply -f k8s/redis-deployment.yaml
kubectl apply -f k8s/postgres-deployment.yaml
kubectl apply -f k8s/migration-job.yaml
kubectl apply -f k8s/nexus-relay-deployment.yaml
kubectl apply -f k8s/prometheus-configmap.yaml
kubectl apply -f k8s/prometheus-deployment.yaml
kubectl apply -f k8s/service.yaml

# Verify rollout
kubectl rollout status deployment/nexus-relay -n nexus
```

### 2. Environment Configuration

#### Database Setup (PostgreSQL)
```bash
# Connection pooling (pgBouncer recommended for 1000+ connections)
# Configuration: pgbouncer.ini
[databases]
nexus = host=postgres.nexus.svc.cluster.local dbname=nexus

# Production settings
max_db_connections = 100
max_client_conn = 2000
pool_mode = transaction
```

#### Redis Cluster (Optional, for high availability)
```bash
# Redis Sentinel configuration
# sentinel.conf
sentinel monitor nexus-master 127.0.0.1 6379 2
sentinel down-after-milliseconds nexus-master 3000
sentinel auth-pass nexus-master ${REDIS_PASSWORD}
```

### 3. TLS Configuration

#### Generate Production Certificates
```bash
# Using Let's Encrypt with Certbot
certbot certonly --dns-cloudflare -d relay.nexus-messaging.io

# Mount into container
docker run -v /etc/letsencrypt:/etc/nexus/certs nexus-relay
```

#### Configure HTTPS
```rust
// Will be automatically detected in main.rs
// From environment: TLS_CERT_PATH and TLS_KEY_PATH
```

### 4. Monitoring & Alerting

#### Prometheus Rules
```yaml
# monitoring/rules.yml - Automatically loaded
groups:
  - name: nexus
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: rate(errors_total[5m]) > 0.05
        for: 5m
        annotations:
          summary: "High error rate detected"
      
      - alert: DatabaseDown
        expr: pg_up == 0
        for: 1m
        annotations:
          summary: "PostgreSQL database is down"
```

#### Grafana Dashboards
Auto-provisioned from `monitoring/grafana/dashboards/`

### 5. Backup & Recovery

#### PostgreSQL Automatic Backups
```bash
# Daily backups to S3
aws s3 sync /var/lib/postgresql/backups s3://nexus-backups/

# Point-in-time recovery
pg_basebackup -h localhost -D /var/lib/postgresql/backup -P -v
```

#### Redis Persistence
```bash
# AOF (Append-Only File) enabled
appendonly yes
appendfsync everysec
```

---

##  Security Hardening

### 1. Network Security
```bash
# Firewall rules (ufw example)
ufw allow 22/tcp      # SSH only
ufw allow 3000/tcp    # API (use behind reverse proxy in prod)
ufw allow 9091/tcp    # Prometheus (internal only)
ufw deny incoming

# Network policies (Kubernetes)
kubectl apply -f k8s/network-policy.yaml
```

### 2. Secrets Management

#### Option A: Kubernetes Secrets
```bash
kubectl create secret generic nexus-secrets \
  --from-literal=db-password=${DB_PASSWORD} \
  --from-literal=redis-password=${REDIS_PASSWORD} \
  --from-literal=tls-key=@certs/key.pem \
  --from-literal=tls-cert=@certs/certificate.pem \
  -n nexus
```

#### Option B: Sealed Secrets
```bash
# Install sealed-secrets controller
kubectl apply -f https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.18.0/controller.yaml

# Seal secrets
kubeseal -f secrets.yaml > sealed-secrets.yaml
```

### 3. Resource Limits
```yaml
# Kubernetes resource quotas prevent DoS
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"
```

---

##  Testing & Validation

### 1. Unit Tests
```bash
make test-release
```

### 2. Load Testing
```bash
# Start with 100 users, ramp to 1000
make load-test
```

### 3. Security Audit
```bash
# Audit dependencies
make security-audit

# Run security tests
cargo test security::
```

### 4. Chaos Engineering
```bash
# Kill random pods
kubectl chaos kill-pod --namespace nexus --match-labels app=nexus-relay

# Verify recovery
kubectl get pods -n nexus -w
```

---

##  Observability

### 1. Health Checks
```bash
# Readiness probe
curl -f http://localhost:3000/health

# Liveness probe (in Kubernetes)
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 10
```

### 2. Metrics
```bash
# Prometheus UI: http://localhost:9091

# Key metrics to monitor:
- nexus_relay_connections_active
- nexus_relay_messages_processed_total
- nexus_relay_errors_total
- nexus_relay_latency_p95_ms
- pg_stat_active_connections
- redis_connected_clients
```

### 3. Logs
```bash
# View relay logs
kubectl logs -f deployment/nexus-relay -n nexus

# View system logs
journalctl -u nexus-relay -f
```

---

##  Maintenance & Updates

### 1. Rolling Updates
```bash
# Kubernetes handles graceful rolling update
kubectl set image deployment/nexus-relay \
  nexus-relay=nexus:0.2.2 \
  -n nexus

# Monitor rollout
kubectl rollout status deployment/nexus-relay -n nexus
```

### 2. Database Migrations
```bash
# Automatic via migration job
kubectl apply -f k8s/migration-job.yaml

# Manual if needed
sqlx migrate run --database-url "${DATABASE_URL}"
```

### 3. Certificate Renewal
```bash
# Automatic with cert-manager (recommended)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Manual renewal
certbot renew --dns-cloudflare
```

---

##  Troubleshooting

### Service won't start
```bash
# Check logs
docker logs nexus-relay

# Verify database connectivity
psql postgres://nexus:${DB_PASSWORD}@localhost:5432/nexus

# Verify Redis connectivity
redis-cli -a ${REDIS_PASSWORD} ping
```

### Memory leak
```bash
# Check memory usage
docker stats nexus-relay

# Restart service (temporary fix)
docker restart nexus-relay

# Contact support if issue persists
```

### High latency
```bash
# Analyze slow queries
# Enable log_statement=all in PostgreSQL and check

# Check connection pool saturation
SELECT count(*) FROM pg_stat_activity;

# Monitor Redis evictions
redis-cli info stats | grep evicted
```

---

##  Support & Escalation

For production support:
- **Security Issues**: security@nexus-messaging.io
- **Infrastructure**: ops@nexus-messaging.io
- **Documentation**: docs@nexus-messaging.io
- **Community Chat**: discord.gg/nexus-messaging

---

**Last Updated**: April 2, 2026  
**Version**: v0.2.1 Production Guide
