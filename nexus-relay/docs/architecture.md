# NEXUS Architecture Diagrams (C4 Model)

## Level 1: System Context

```mermaid
C4Context
    title System Context Diagram - NEXUS Messenger

    Person(user, "NEXUS User", "End user of the messaging platform")
    Person(admin, "System Administrator", "Manages and monitors the platform")
    
    System(nexus, "NEXUS Messaging Platform", "Post-quantum secure messaging with sealed sender")
    
    System_Ext(monitoring, "Monitoring Stack", "Prometheus, Grafana, Loki")
    System_Ext(alerting, "Alerting System", "PagerDuty, Slack")
    System_Ext(cdn, "CDN/WAF", "CloudFlare, AWS WAF")
    
    Rel(user, nexus, "Sends/receives encrypted messages", "WebSocket/WSS")
    Rel(admin, monitoring, "Monitors platform health", "HTTPS")
    Rel(nexus, monitoring, "Exports metrics", "Prometheus")
    Rel(monitoring, alerting, "Sends alerts", "Webhook")
    Rel(cdn, nexus, "Proxies traffic", "HTTPS")
```

## Level 2: Container Diagram

```mermaid
C4Container
    title Container Diagram - NEXUS Messaging Platform

    Person(user, "NEXUS User")
    
    System_Boundary(nexus, "NEXUS Platform") {
        Container(relay, "Relay Server", "Rust, Axum", "Routes encrypted messages, never sees content")
        Container(web, "Web Client", "React, TypeScript", "Browser-based messaging")
        Container(android, "Android App", "Kotlin, Jetpack Compose", "Native Android client")
        Container(ios, "iOS App", "Swift, SwiftUI", "Native iOS client")
        Container(desktop, "Desktop App", "Tauri, Rust", "Cross-platform desktop")
        ContainerDb(postgres, "PostgreSQL", "PostgreSQL 16", "User data, prekeys, messages")
        ContainerDb(redis, "Redis", "Redis 7", "Session state, caching")
        ContainerDb(s3, "Object Storage", "S3/MinIO", "Media files, attachments")
    }
    
    Rel(user, web, "Uses", "HTTPS")
    Rel(user, android, "Uses", "HTTPS")
    Rel(user, ios, "Uses", "HTTPS")
    Rel(user, desktop, "Uses", "HTTPS")
    Rel(web, relay, "Connects", "WSS")
    Rel(android, relay, "Connects", "WSS")
    Rel(ios, relay, "Connects", "WSS")
    Rel(desktop, relay, "Connects", "WSS")
    Rel(relay, postgres, "Reads/writes", "TCP")
    Rel(relay, redis, "Caches", "TCP")
    Rel(relay, s3, "Stores media", "HTTPS")
```

## Level 3: Component Diagram - Relay Server

```mermaid
C4Component
    title Component Diagram - NEXUS Relay Server

    Container_Boundary(relay, "Relay Server") {
        Component(ws_handler, "WebSocket Handler", "Axum", "Manages WebSocket connections")
        Component(auth, "Authentication", "Rust", "Identity verification, challenge-response")
        Component(router, "Message Router", "Rust", "Routes sealed messages to recipients")
        Component(prekey_mgr, "Prekey Manager", "Rust", "Manages one-time prekeys")
        Component(group_mgr, "Group Manager", "Rust", "Group chat management")
        Component(call_mgr, "Call Manager", "Rust", "WebRTC signaling")
        Component(rate_limiter, "Rate Limiter", "Rust", "Per-IP rate limiting")
        Component(threat_det, "Threat Detection", "Rust/ML", "Anomaly detection")
        Component(dp_engine, "DP Engine", "Rust", "Differential privacy for metadata")
        Component(metrics, "Metrics Exporter", "Prometheus", "Exports metrics")
    }
    
    Rel(ws_handler, auth, "Verifies identity")
    Rel(ws_handler, router, "Routes messages")
    Rel(router, prekey_mgr, "Fetches prekeys")
    Rel(router, group_mgr, "Handles groups")
    Rel(ws_handler, rate_limiter, "Checks limits")
    Rel(ws_handler, threat_det, "Reports events")
    Rel(threat_det, dp_engine, "Anonymizes data")
    Rel(metrics, ws_handler, "Collects metrics")
```

## ERD Diagram

```mermaid
erDiagram
    USERS ||--o{ ONE_TIME_PREKEYS : has
    USERS ||--|| PREKEY_BUNDLES : owns
    USERS ||--o{ OFFLINE_MESSAGES : receives
    USERS ||--o{ GROUP_MEMBERS : joins
    USERS ||--o{ CALL_SESSIONS : participates
    USERS ||--o{ DELIVERY_RECEIVES : receives
    
    GROUPS ||--o{ GROUP_MEMBERS : contains
    GROUPS ||--o{ GROUP_MESSAGES : contains
    
    CONVERSATIONS ||--o{ OFFLINE_MESSAGES : tracks
    
    USERS {
        uuid id PK
        varchar recipient_hash UK
        bytea identity_key
        bytea signed_prekey
        bytea signed_prekey_signature
        timestamp created_at
        timestamp last_seen
        enum status
        bigint dp_request_count
    }
    
    ONE_TIME_PREKEYS {
        uuid id PK
        uuid user_id FK
        integer prekey_index
        bytea prekey_data
        boolean is_used
        timestamp used_at
    }
    
    OFFLINE_MESSAGES {
        uuid id PK
        varchar recipient_hash
        bytea sealed_content
        varchar sender_hash
        timestamp received_at
        timestamp expires_at
        enum priority
        boolean is_delivered
        integer dp_size_bucket
    }
    
    GROUPS {
        uuid id PK
        varchar name
        varchar owner_hash
        bigint epoch
        integer max_members
        boolean is_public
    }
    
    GROUP_MEMBERS {
        uuid group_id FK
        varchar member_hash
        boolean is_admin
        timestamp joined_at
    }
    
    CALL_SESSIONS {
        uuid id PK
        varchar initiator_hash
        varchar recipient_hash
        enum call_type
        enum status
        timestamp started_at
        timestamp ended_at
        integer duration_secs
    }
    
    SECURITY_ALERTS {
        uuid id PK
        enum severity
        varchar alert_type
        text description
        varchar user_hash
        inet ip_address
        float anomaly_score
        jsonb features
        timestamp created_at
    }
```

## STRIDE Threat Model

```mermaid
graph TD
    subgraph "STRIDE Analysis - NEXUS"
        S[Spoofing] --> S1[Identity impersonation]
        S --> S2[Man-in-the-middle]
        S --> S3[Prekey substitution]
        
        T[Tampering] --> T1[Message modification]
        T --> T2[Key manipulation]
        T --> T3[Database corruption]
        
        R[Repudiation] --> R1[Message denial]
        R --> R2[Action denial]
        
        I[Information Disclosure] --> I1[Metadata leakage]
        I --> I2[Key extraction]
        I --> I3[Traffic analysis]
        
        D[Denial of Service] --> D1[Connection flood]
        D --> D2[Message spam]
        D --> D3[Resource exhaustion]
        
        E[Elevation of Privilege] --> E1[Admin access]
        E --> E2[Group privilege escalation]
        E --> E3[Session hijacking]
    end
    
    subgraph "Mitigations"
        S1 --> M1[X3DH identity binding]
        S2 --> M2[TLS + certificate pinning]
        S3 --> M3[Prekey signatures]
        
        T1 --> M4[AEAD encryption]
        T2 --> M5[Secure key storage]
        T3 --> M6[Database integrity checks]
        
        I1 --> M7[Sealed sender + DP]
        I2 --> M8[Hardware security modules]
        I3 --> M9[Traffic padding]
        
        D1 --> M10[Rate limiting]
        D2 --> M11[Message quotas]
        D3 --> M12[Resource limits]
        
        E1 --> M13[RBAC + audit logging]
        E2 --> M14[Group permission checks]
        E3 --> M15[Session binding]
    end
```

## Data Flow Diagram (DFD)

```mermaid
graph LR
    subgraph "Sender Side"
        S[User A] --> SE[Encrypt Message]
        SE --> SP[Seal with Recipient Key]
    end
    
    subgraph "Relay Server"
        SP --> RR[Receive Sealed Message]
        RR --> RS[Route to Recipient]
        RS --> RO[Queue if Offline]
    end
    
    subgraph "Recipient Side"
        RS --> RU[User B Receives]
        RU --> RD[Decrypt with Private Key]
        RD --> RP[Read Plaintext]
    end
    
    subgraph "Key Exchange"
        X3DH[X3DH Handshake] --> SS[Shared Secret]
        SS --> DR[Double Ratchet]
        DR --> MK[Message Keys]
    end
    
    MK --> SE
    MK --> RD
```

## Deployment Diagram

```mermaid
graph TB
    subgraph "Internet"
        U[Users]
    end
    
    subgraph "CDN/WAF"
        CF[CloudFlare]
    end
    
    subgraph "Kubernetes Cluster"
        subgraph "Ingress"
            NG[Nginx Ingress]
        end
        
        subgraph "Application"
            R1[Relay Pod 1]
            R2[Relay Pod 2]
            R3[Relay Pod N]
        end
        
        subgraph "Data"
            PG[(PostgreSQL)]
            RD[(Redis)]
            S3[(MinIO)]
        end
        
        subgraph "Monitoring"
            PR[Prometheus]
            GF[Grafana]
        end
    end
    
    U --> CF
    CF --> NG
    NG --> R1
    NG --> R2
    NG --> R3
    R1 --> PG
    R2 --> PG
    R3 --> PG
    R1 --> RD
    R2 --> RD
    R3 --> RD
    R1 --> PR
    R2 --> PR
    R3 --> PR
    PR --> GF
```
