# NEXUS Threat Model v0.2.1

## 1. Overview

This document defines the threat model for the NEXUS secure messaging platform.
It identifies assets, threats, attack vectors, and mitigations.

## 2. System Assets

### 2.1 High-Value Assets
| Asset | Sensitivity | Location |
|-------|-------------|----------|
| User Identity Keys | CRITICAL | Client-side (Secure Enclave) |
| Message Content | CRITICAL | End-to-end encrypted |
| Prekey Bundles | HIGH | Relay server (encrypted) |
| Session State | HIGH | Client-side |
| Message Metadata | MEDIUM | Relay server |
| Server Logs | LOW | Server-side |

### 2.2 Security Properties

| Property | Definition | Implementation |
|----------|------------|----------------|
| **Confidentiality** | Only intended recipient reads message | X3DH + Double Ratchet + AES-256-GCM |
| **Integrity** | Messages cannot be tampered | HMAC-SHA256 in Double Ratchet |
| **Authentication** | Verify sender identity | Ed25519 signatures |
| **Forward Secrecy** | Past messages safe if key compromised | Double Ratchet key rotation |
| **Post-Compromise Security** | Future messages safe after recovery | Double Ratchet self-healing |
| **Deniability** | Cannot prove authorship to 3rd party | No persistent signatures on messages |

## 3. Threat Actors

### 3.1 Passive Adversaries
- **Nation-state surveillance**: Traffic analysis, metadata collection
- **ISP/Network monitoring**: Packet inspection, timing analysis
- **Compromised relay**: Metadata access, message routing

### 3.2 Active Adversaries
- **Malicious users**: Message injection, impersonation
- **Compromised device**: Key extraction, message access
- **Man-in-the-middle**: Session hijacking, key substitution

### 3.3 Insider Threats
- **Server operators**: Metadata access, traffic analysis
- **Developers**: Backdoor insertion, key leakage

## 4. Attack Vectors & Mitigations

### 4.1 Key Exchange Attacks

| Attack | Description | Mitigation | Status |
|--------|-------------|------------|--------|
| Key Substitution | Replace public key during exchange | Verify signatures on prekeys | ✅ Implemented |
| Replay Attack | Reuse old key exchange messages | One-time prekeys, ephemeral keys | ✅ Implemented |
| Unknown Key Share | A thinks talking to B, actually C | Identity binding in X3DH | ⚠️ Partial |
| Key Compromise | Attacker obtains private key | Forward secrecy via ratchet | ✅ Implemented |

### 4.2 Message Attacks

| Attack | Description | Mitigation | Status |
|--------|-------------|------------|--------|
| Message Injection | Send fake messages | AEAD authentication | ✅ Implemented |
| Message Replay | Resend old messages | Message numbering, nonces | ✅ Implemented |
| Message Reorder | Change message sequence | Sequence numbers | ✅ Implemented |
| Message Drop | Prevent delivery | Offline queue, delivery receipts | ✅ Implemented |

### 4.3 Metadata Attacks

| Attack | Description | Mitigation | Status |
|--------|-------------|------------|--------|
| Traffic Analysis | Infer communication patterns | Sealed sender | ✅ Implemented |
| Timing Analysis | Correlate send/receive times | Padding, delays | ⚠️ Partial |
| Volume Analysis | Message size patterns | Padding to fixed sizes | ⚠️ Partial |
| Social Graph | Who talks to whom | Sealed sender | ✅ Implemented |

### 4.4 Server-Side Attacks

| Attack | Description | Mitigation | Status |
|--------|-------------|------------|--------|
| DoS/DDoS | Overwhelm server | Rate limiting | ✅ Implemented |
| SQL Injection | Malicious queries | Parameterized queries | ✅ N/A (no SQL) |
| Buffer Overflow | Crash server, RCE | Rust memory safety | ✅ Guaranteed |
| Privilege Escalation | Gain admin access | Access control | ⚠️ Basic |

## 5. Known Limitations

### 5.1 Cryptographic Limitations
1. **No formal verification**: Cryptographic implementation not formally verified
2. **Simplified DH**: Desktop client uses simplified DH (not production X25519)
3. **Kyber stubs**: Post-quantum KEM uses random bytes in some clients
4. **No constant-time guarantees**: Not verified against timing attacks

### 5.2 Metadata Limitations
1. **Timing leakage**: Message timing not fully obscured
2. **Size leakage**: Message sizes partially observable
3. **Connection patterns**: Server knows when users connect
4. **No mixnet**: Direct connections, not routed through mixnet

### 5.3 Infrastructure Limitations
1. **Single server**: No federation or clustering
2. **In-memory state**: Server restart loses state
3. **No backup**: No disaster recovery
4. **No monitoring**: Limited observability

## 6. Security Assumptions

### 6.1 Trusted Components
- Client device is not compromised
- Secure Enclave / Keystore provides hardware protection
- Operating system provides process isolation
- Network provides basic connectivity

### 6.2 Untrusted Components
- Relay server (treated as honest-but-curious)
- Network infrastructure (may observe traffic)
- Other users (may be malicious)
- Cloud infrastructure (may be compromised)

## 7. Incident Response

### 7.1 Vulnerability Disclosure
- Email: security@nexus.example.com (TODO: set up)
- PGP Key: TODO
- Response time: 90 days for non-critical, 30 days for critical

### 7.2 Key Compromise Procedure
1. Revoke compromised identity
2. Generate new identity keys
3. Notify contacts via out-of-band channel
4. Re-establish sessions with new keys

## 8. Future Improvements

### 8.1 Short-term (3 months)
- [ ] Formal verification of Double Ratchet
- [ ] Constant-time crypto operations
- [ ] Fuzzing infrastructure
- [ ] Comprehensive audit logging

### 8.2 Medium-term (6 months)
- [ ] External security audit
- [ ] Federated server support
- [ ] Mixnet integration research
- [ ] Post-quantum crypto production implementation

### 8.3 Long-term (12+ months)
- [ ] Formal security proofs
- [ ] Hardware security module integration
- [ ] Zero-knowledge metadata proofs
- [ ] Compliance certifications (SOC2, ISO27001)

## 9. References

1. Signal Protocol Specification: https://signal.org/docs/
2. Double Ratchet Algorithm: https://signal.org/docs/specifications/doubleratchet/
3. X3DH Key Agreement: https://signal.org/docs/specifications/x3dh/
4. CRYSTALS-Kyber: https://pq-crystals.org/kyber/
5. OWASP Threat Modeling: https://owasp.org/www-community/Threat_Modeling

## 10. Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 0.1 | 2026-04-02 | NEXUS Team | Initial threat model |
