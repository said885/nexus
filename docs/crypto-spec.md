# Post-Quantum Cryptography Specification (v0.3.0)

## KEM (Key Encapsulation Mechanism)
Algorithm: **Kyber1024** (NIST FIPS 203)
Implementation: `nexus-crypto/src/pq.rs`
Usage: Post-quantum session key exchange in combination with X25519.

## Digital Signatures
Algorithm: **Dilithium5** (NIST FIPS 204)
Implementation: `nexus-crypto/src/identity.rs`
Usage: Identity verification and message authentication.

## Symmetric Encryption
Algorithm: **ChaCha20-Poly1305**
Usage: Bulk data encryption for all message content.

## Key Derivation
KDF: **HKDF-SHA256**
Usage: Deriving session keys from shared secrets.
