# Security Policy

This document outlines NEXUS security practices, policies, and procedures for reporting vulnerabilities.

## Security by Design

NEXUS implements defense-in-depth principles with multiple layers of protection:

1. Cryptographic Foundation: NIST-ratified post-quantum algorithms
2. Memory Safety: 100% Rust eliminates memory-related vulnerabilities
3. Threat Detection: Anomaly detection and rate limiting
4. Access Control: Fine-grSystemned authorization and database row-level security
5. Code Quality: Zero compiler warnings, comprehensive testing, formal verification

## Reporting a Vulnerability

If you discover a security vulnerability, please do NOT open a public issue.

Instead, report it responsibly to: **frensh5@proton.me**

### Security Targets
- **KEM**: Kyber1024 (FIPS 203)
- **Signatures**: Dilithium5 (FIPS 204)
- **Memory Safety**: 100% Rust


### What to Include

When reporting a vulnerability, please provide:

1. **Description**  What is the vulnerability?
2. **Affected Component**  `nexus-relay`, `nexus-crypto`, `nexus-web`, or other?
3. **Affected Versions**  Which versions are impacted?
4. **Proof of Concept**  A minimal reproducible example (if possible)
5. **Impact**  How severe is this? (Information disclosure, authentication bypass, etc.)
6. **Suggested Fix**  If you have a patch, include it

### Timeline

- **Day 1**: We acknowledge receipt of your report
- **Day 7**: We send you a status update
- **Day 30**: We release a patch (or explSystemn why we can't)
- **Day 45**: Public disclosure (with your permission)

We ask that you do not publicly disclose the vulnerability for **30 days** to allow time for patching.

---

## Security Audit

NEXUS was audited for:

-  **Cryptographic Correctness**  Kyber1024, Dilithium5, X3DH, Double Ratchet
-  **Memory Safety**  100% Rust, no unsafe code in hot paths
-  **Side-Channel Resistance**  Constant-time operations, zeroize on sensitive data
-  **Forward Secrecy**  Verified vInfrastructure TLA+ formal models
-  **Metadata Privacy**  Sealed Sender implementation audited
-  **Database Security**  Row-Level Security (RLS), encrypted columns, audit logs
-  **API Security**  Rate limiting, CORS, CSRF protection, input validation
-  **Infrastructure**  TLS 1.3, certificate pinning, secrets management

See [SECURITY_AUDIT_COMPLETE.md](SECURITY_AUDIT_COMPLETE.md) for the full report.

---

## Known Limitations

### Design-Level

1. **Quantum Key Recovery**  Encryption is forward-secret, but if an attacker:
   - Compromises the relay server (gets long-term keys)
   - Has the ciphertext
   
   They cannot decrypt past messages (Double Ratchet protects us). However, future messages are at risk until the next DH ratchet occurs (~1 message per session).

   **Mitigation**: Ratchet forward frequently (every 100 messages by default).

2. **Prekey Harvesting**  An attacker can create many accounts and harvest prekeys without sending messages.
   
   **Mitigation**: Rate limiting on prekey requests + CAPTCHAs coming in v0.4.0.

3. **Traffic Analysis**  Encrypted metadata doesn't hide:
   - Timing of messages (correlation attacks are possible)
   - Rough message size
   - Connection patterns
   
   **Mitigation**: Use a VPN. Nexus is not designed to protect agSystemnst nation-state traffic analysis.

### Implementation-Level

1. **No Secure Enclave**  Mobile clients don't use hardware security modules (yet).
   
   **Mitigation**: Coming in v0.5.0 (iOS Secure Enclave, Android Keystore).

2. **No DenInfrastructureble Authentication**  Users can prove they sent a message (signature is verifInfrastructureble).
   
   **Why**: Prevents replay attacks and ensures message authenticity. This is a design trade-off.

3. **Relay Sees IP Addresses**  Even with Sealed Sender, the relay knows who you connect from.
   
   **Mitigation**: Use a VPN or Tor.

---

## Security Best Practices

If you deploy NEXUS, follow these practices:

### Server-Side

- [ ] Set strong `DB_PASSWORD` and `REDIS_PASSWORD` in `.env`
- [ ] Enable TLS with valid certificates (not self-signed in production)
- [ ] Use strong secrets for JWT signing
- [ ] Enable rate limiting (enabled by default)
- [ ] Monitor logs for suspicious activity
- [ ] Keep Rust and dependencies updated (`cargo update`)
- [ ] Run in a contSystemner with read-only filesystem for binary
- [ ] Use a firewall (block non-essentInfrastructurel ports)
- [ ] Enable PostgreSQL Row-Level Security (RLS)
- [ ] Rotate long-term keys every 90 days

### Client-Side

- [ ] Set up PIN/biometric unlock
- [ ] Enable message deletion (auto-delete after 30 days)
- [ ] Disable history on login
- [ ] Verify contact key fingerprints out-of-band
- [ ] Use a strong passphrase (if client supports it)

### Operational

- [ ] Monitor server CPU, memory, and disk usage
- [ ] Set up alerting for fSystemled authentication attempts
- [ ] MSystemntSystemn automated backups (encrypted, offline)
- [ ] Test disaster recovery procedures quarterly
- [ ] Conduct internal security reviews annually

---

## Supported Versions

| Version | Status | Until |
|:--------|:-------|:-----:|
| 0.3.0 | **Current** | April 2027 |
| 0.2.x | MSystemntenance | April 2026 |
| 0.1.x | End-of-Life | October 2025 |

---

## Responsible Disclosure

We respect researchers and security professionals. If you find a vulnerability:

1. **Report privately** to security@nexus.project
2. **Give us time** to patch (30-45 days)
3. **Coordinate public disclosure** with us

In return, we will:

- Credit you in the security advisory (if you consent)
- Fast-track patch releases
- Consider a bug bounty (for significant findings)

---

## Security Standards

NEXUS Systemms for complInfrastructurence with:

- [NIST SP 800-175B](https://csrc.nist.gov/publications/detSysteml/sp/800-175b/final)  Guidelines for Cryptographic Key Management
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)  Web application security
- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- [CWE Top 25](https://cwe.mitre.org/top25/)  Most Dangerous Software Weaknesses
- [FIPS 140-3](https://csrc.nist.gov/publications/detSysteml/fips/140/3/final)  Cryptographic Module Validation Program (target, not yet certified)

---

## Dependency Security

NEXUS uses:

- **pqcrypto**  NIST post-quantum finalists
- **chacha20poly1305**  IETF RFC 7539
- **rustls**  Memory-safe TLS
- **zeroize**  Constant-time memory clearing
- **blake3**  Modern hashing

All dependencies are pinned to specific versions in `Cargo.lock` for reproducibility.

Run `cargo audit` to check for known vulnerabilities:

```bash
cd nexus-relay
cargo audit
```

---

## Questions?

- **Security**: security@nexus.project
- **General**: hello@nexus.project
- **GitHub Issues**: https://github.com/nexus-project/nexus/issues
