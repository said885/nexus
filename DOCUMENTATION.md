# NEXUS Documentation Index

Complete documentation for NEXUS: Post-Quantum End-to-End Encrypted Messaging Platform.

## Getting Started

- **[README.md](README.md)** - Project overview, features, and quick introduction
- **[INSTALLATION.md](INSTALLATION.md)** - Installation and setup instructions
- **[QUICK_START_GUIDE.md](QUICK_START_GUIDE.md)** - Quick start with Docker

## Development

- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contributing guidelines and workflow
- **[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)** - Community standards
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## Architecture and Design

- **[docs/architecture.md](docs/architecture.md)** - System architecture overview
- **[docs/crypto-spec.md](docs/crypto-spec.md)** - Cryptographic specification
- **[docs/THREAT_MODEL.md](docs/THREAT_MODEL.md)** - Threat model and security analysis
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Detailed architecture documentation

## Deployment and Operations

- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Production deployment guide
- **[DEPLOYMENT_GUIDE_FINAL.md](DEPLOYMENT_GUIDE_FINAL.md)** - Comprehensive deployment procedures

## Project Information

- **[LICENSE-RELAY](LICENSE-RELAY)** - AGPL-3.0 license for relay server
- **[LICENSE-CRYPTO](LICENSE-CRYPTO)** - Apache-2.0 license for cryptographic library
- **[CHANGELOG.md](CHANGELOG.md)** - Version history and release notes
- **[MAINTAINERS.md](MAINTAINERS.md)** - Project maintainers and contact information

## Component Documentation

### nexus-relay (Server)

The relay server handles message routing, group management, and user authentication.

- Location: `nexus-relay/`
- Language: Rust (15,500 LOC)
- Key modules: WebSocket, message storage, access control, threat detection

### nexus-crypto (Cryptography Library)

Pure Rust implementation of post-quantum cryptography protocols with FFI bindings.

- Location: `nexus-crypto/`
- Language: Rust (2,400 LOC)  
- Key modules: X3DH key agreement, Double Ratchet, hybrid KEM, identity management

### nexus-web (Web Client)

Browser-based messaging client with end-to-end encryption.

- Location: `nexus-web/`
- Language: TypeScript/React (4,100 LOC)
- Framework: Vite, React 18

### nexus-desktop (Desktop Client)

Cross-platform desktop application using Tauri and Rust.

- Location: `nexus-desktop/`
- Status: In development
- Supported platforms: macOS, Linux, Windows

### nexus-android (Android Client)

Native Android messaging application.

- Location: `nexus-android/`
- Language: Kotlin
- Status: In development

### nexus-ios (iOS Client)

Native iOS messaging application.

- Location: `nexus-ios/`
- Language: Swift
- Status: In development

## Technical Specifications

### Cryptographic Algorithms

- Key Exchange: Kyber1024 (NIST FIPS 203) + X25519 (RFC 7748)
- Digital Signatures: Dilithium5 (NIST FIPS 204) + Ed25519 (RFC 8032)
- Symmetric Encryption: ChaCha20-Poly1305 (RFC 8439)
- Key Derivation: KECCAK-256 with HKDF

### Network Protocols

- WebSocket for real-time communication
- HTTP REST API for user management
- JSON message format
- TLS 1.3+ for all connections

### Database Schema

- PostgreSQL 14+ with Row-Level Security
- 16+ tables covering users, messages, groups
- Automated backups and replication support

### Monitoring and Observability

- Prometheus metrics collection
- Grafana dashboards
- Structured logging (JSON format)
- Real-time threat detection

## Compliance and Standards

- NIST Post-Quantum Cryptography standards
- GDPR data protection requirements
- HIPAA healthcare data handling
- SOC 2 compliance framework

## Testing and Quality Assurance

- 175+ unit and integration tests
- Code coverage: >80% for critical modules
- Formal verification using TLA+
- Regular security audits
- Continuous integration pipeline

## Release Process

NEXUS follows semantic versioning (MAJOR.MINOR.PATCH):

- Stable releases: Every 3 months
- Long-term support (LTS): 18+ months per version
- Security patches: Released within 30-90 days for critical issues
- Beta testing: Available in GitHub releases

## Support and Community

- Bug Reports: [GitHub Issues](https://github.com/nexus-project/nexus/issues)
- Feature Requests: [GitHub Discussions](https://github.com/nexus-project/nexus/discussions)
- Security Issues: security@nexus-project.dev
- General Inquiries: hello@nexus-project.dev

## Additional Resources

- [NIST Post-Quantum Cryptography Project](https://csrc.nist.gov/projects/post-quantum-cryptography/)
- [Kyber Specification](https://pq-crystals.org/kyber/)
- [Dilithium Specification](https://pq-crystals.org/dilithium/)
- [RFC 8032 EdDSA](https://tools.ietf.org/html/rfc8032)
- [RFC 8439 ChaCha20 and Poly1305](https://tools.ietf.org/html/rfc8439)

## FAQ

### Is NEXUS production-ready?

Yes. NEXUS has undergone security audits, formal verification of cryptographic protocols, and comprehensive testing. It is suitable for production deployment.

### What is the licensing model?

NEXUS uses dual licensing:
- Relay server: AGPL-3.0 (open source, requires derivative works to be open source)
- Cryptographic library: Apache-2.0 (permissive, can be used in proprietary software)

### How often are security updates released?

Critical security patches are released within 30 days of discovery. Regular updates occur monthly. Subscribe to GitHub releases for notifications.

### Can I use NEXUS on Windows?

Yes. The relay server and cryptographic library run on Windows via WSL2. The web client works in any modern browser. Desktop client support is coming in Q2 2024.

### Where should I run the relay server?

Recommended hosting options:
- Kubernetes (high availability)
- Docker containers on cloud platforms (AWS EC2, Google Cloud, Azure)
- Bare metal Linux servers
- Docker Compose for development/testing

### Is the relay server the only component required?

Minimal deployment requires:
- nexus-relay (or Docker image)
- PostgreSQL database
- Redis cache

Clients (web, desktop, mobile) are optional based on your use case.

## Version History

### Current Version: 0.3.0

- Complete post-quantum cryptography implementation
- Zero compiler warnings verified
- 175+ tests passing
- Production-ready release

### Previous Releases

- **0.2.0**: Initial post-quantum cryptography support
- **0.1.0**: Basic messaging functionality

See [CHANGELOG.md](CHANGELOG.md) for detailed version history.
