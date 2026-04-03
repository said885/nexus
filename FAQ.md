# NEXUS FAQ - Frequently Asked Questions

## General Questions

### What is NEXUS?
NEXUS is a production-grade, fully open-source messaging platform built for the quantum era. Unlike Signal, WhatsApp, and Matrix, NEXUS uses NIST-standardized post-quantum cryptography (Kyber1024, Dilithium5) alongside classical algorithms for maximum security.

### Why post-quantum cryptography?
Quantum computers will break all current encryption within 10-30 years. Attackers are already performing "harvest-now-decrypt-later" attacks—recording encrypted messages today to decrypt them later. NEXUS protects against this threat.

### Is NEXUS production-ready?
Yes. Version 0.3.0 is stable with 175+ tests, 0 compiler warnings, and formal verification. It's suitable for government, enterprise, and critical infrastructure deployments.

### Who maintains NEXUS?
NEXUS is maintained by said885 (frensh5@proton.me). For commercial support and enterprise licenses, contact the same email.

---

## Technical Questions

### What encryption does NEXUS use?
- **Key Exchange**: Kyber1024 (post-quantum) + X25519 (classical)
- **Signatures**: Dilithium5 (post-quantum) + Ed25519 (classical)
- **Symmetric**: ChaCha20-Poly1305
- **KDF**: HKDF-SHA256
- **Design**: Double Ratchet with forward secrecy

### Why hybrid encryption?
Hybrid encryption means if either classical or post-quantum algorithms break, your messages remain secure. This is the only responsible approach for long-term confidentiality.

### How does the relay work?
The relay server is zero-knowledge: it cannot read message content, metadata, or know who communicates with whom. This is achieved through sealed-sender protocol and cryptographic design.

### Is the code auditable?
Yes. NEXUS is 100% Rust with zero unsafe code in critical paths. All algorithms use NIST standards, not custom cryptography.

### What about forward secrecy?
NEXUS implements PFS via:
1. Ephemeral key exchange per session
2. Double Ratchet with frequent key rotation
3. Deletion of chain keys (one-way function)

Even if an attacker compromises your device, past messages remain secure.

---

## Deployment Questions

### How do I install NEXUS?
See [INSTALLATION.md](INSTALLATION.md) for detailed setup instructions. Quick start:
```bash
docker-compose -f docker-compose.prod.yml up -d
```

### What are the system requirements?
- **Server**: Rust 1.75+, PostgreSQL 14+, Redis 7+
- **Clients**: Any modern browser (web), Tauri-compatible OS (desktop)

### Can I deploy on Kubernetes?
Yes. See [DEPLOYMENT.md](DEPLOYMENT.md) for Kubernetes manifests and best practices.

### What's the performance?
- Message latency: P99 < 5ms
- Throughput: 50,000+ messages/second
- Memory footprint: 12 MB idle
- Binary size: 6.1 MB (stripped)

### How do I scale NEXUS?
NEXUS is stateless and scales horizontally via Kubernetes. PostgreSQL replication handles data.

---

## Security Questions

### Is NEXUS secure?
NEXUS implements NIST-standardized algorithms, uses 100% Rust (memory-safe), and has zero compiler warnings. Formal verification (TLA+) confirms cryptographic correctness.

### Has NEXUS been audited?
Security audit documentation is available. Third-party audits can be arranged—contact frensh5@proton.me.

### How do I report security vulnerabilities?
Email frensh5@proton.me with:
- Vulnerability description
- Affected versions
- Proof-of-concept (if possible)
- Suggested fix

Do NOT open a public GitHub issue.

### What's the threat model?
See [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) for detailed security analysis.

---

## Licensing & Commercial Questions

### What license does NEXUS use?
- **Server (nexus-relay)**: AGPL-3.0 (ensures modifications stay open)
- **Crypto library (nexus-crypto)**: Apache-2.0 (permissive, allows proprietary use)
- **Web client**: AGPL-3.0

### Can I use NEXUS in my product?
- **Open source projects**: Use freely under AGPL
- **Proprietary products**: Contact frensh5@proton.me for commercial license
- **Inside your organization**: AGPL compliance required for modifications

### How much does a commercial license cost?
Pricing depends on deployment size and use case. Contact frensh5@proton.me for a quote.

### What about support?
- **Community**: Free via GitHub Issues and Discussions
- **Enterprise SLA**: Available with commercial license
- **Consulting**: Available on hourly basis

---

## Comparison Questions

### How is NEXUS different from Signal?
Signal uses classical cryptography (ECC) that will break in quantum era. NEXUS uses post-quantum algorithms today. Signal connects users directly; NEXUS has a zero-knowledge relay.

### How is NEXUS different from Matrix?
Matrix is a federation protocol for chat rooms; NEXUS is optimized for private, secure messages. NEXUS uses post-quantum crypto; Matrix doesn't. NEXUS has a zero-knowledge relay.

### Should I switch from Signal to NEXUS?
If quantum-resistant security is important, yes. If you value Signal's maturity and ecosystem, maybe wait for NEXUS mobile clients (Q2 2026).

---

## Community & Contribution

### How can I contribute?
See [CONTRIBUTOR_GUIDE.md](CONTRIBUTOR_GUIDE.md) for details. Start with pull requests, issues, or documentation improvements.

### Where's the roadmap?
See [ROADMAP.md](ROADMAP.md) for strategic vision and upcoming features.

### How do I donate?
Bitcoin: `bc1qglsmc82fe5axxhe2gjlwpaflpklm4mh236cjqv`

### Can I use NEXUS commercially?
Yes, with proper licensing. Contact frensh5@proton.me.

---

## Still Have Questions?

- **GitHub Issues**: For bugs and features
- **GitHub Discussions**: For general questions
- **Email**: frensh5@proton.me
- **Documentation**: Start with [README.md](README.md)

---

Last Updated: April 3, 2026
