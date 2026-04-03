// Copyright (c) 2026 said885 <frensh5@proton.me>
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Identity key management for NEXUS.
//!
//! Each identity bundles:
//!  - A Dilithium5 signing key (post-quantum ML-DSA)
//!  - An Ed25519 signing key (classic, for hybrid signatures)
//!  - A Hybrid (Kyber+X25519) KEM key (for key exchange)
//!
//! Hybrid signatures are produced as: dilithium_sig || ed25519_sig,
//! length-prefixed so each can be extracted unambiguously.

use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey};
use pqcrypto_traits::sign::PublicKey as SignPublicKey;
use rand::rngs::OsRng;

use crate::error::{NexusError, Result};
use crate::hybrid_kem::{HybridKeyPair, HybridPublicKey};
use crate::pq::{
    dilithium_sign, dilithium_verify, DilithiumKeyPair, DilithiumPublicKey,
};

// ── Identity key pair ─────────────────────────────────────────────────────────

/// Full identity key pair (signing + KEM).
pub struct IdentityKeyPair {
    /// Post-quantum Dilithium5 signing key.
    pub signing: DilithiumKeyPair,
    /// Classic Ed25519 signing key (hybrid).
    pub signing_classic: SigningKey,
    /// Hybrid KEM key pair for key exchange.
    pub kem: HybridKeyPair,
}

impl IdentityKeyPair {
    /// Generate a fresh identity key pair.
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let signing = DilithiumKeyPair::generate();
        let signing_classic = SigningKey::generate(&mut rng);
        let kem = HybridKeyPair::generate(&mut rng);
        Self {
            signing,
            signing_classic,
            kem,
        }
    }

    /// Derive the public portion of this identity.
    pub fn public_key(&self) -> IdentityPublicKey {
        IdentityPublicKey {
            signing_pq: self.signing.public,
            signing_classic: self.signing_classic.verifying_key(),
            kem: HybridPublicKey {
                kyber: self.kem.public.kyber,
                x25519: self.kem.public.x25519,
            },
        }
    }

    /// Produce a hybrid signature: `u32_le(dil_len) || dil_sig || ed25519_sig`.
    pub fn sign(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let dil_sig = dilithium_sign(&self.signing.secret, msg)?;
        let ed_sig = self.signing_classic.sign(msg);
        let ed_sig_bytes = ed_sig.to_bytes();

        let dil_len = dil_sig.len() as u32;
        let mut out = Vec::with_capacity(4 + dil_sig.len() + ed_sig_bytes.len());
        out.extend_from_slice(&dil_len.to_le_bytes());
        out.extend_from_slice(&dil_sig);
        out.extend_from_slice(&ed_sig_bytes);
        Ok(out)
    }

    /// BLAKE3 hash of the public key material (identity fingerprint).
    pub fn identity_hash(&self) -> [u8; 32] {
        self.public_key().identity_hash()
    }
}

// ── Identity public key ───────────────────────────────────────────────────────

/// The public half of an identity key pair.
pub struct IdentityPublicKey {
    pub signing_pq: DilithiumPublicKey,
    pub signing_classic: VerifyingKey,
    pub kem: HybridPublicKey,
}

impl IdentityPublicKey {
    /// Verify a hybrid signature (both Dilithium5 and Ed25519 must pass).
    pub fn verify(&self, msg: &[u8], sig: &[u8]) -> Result<()> {
        if sig.len() < 4 {
            return Err(NexusError::IntegrityFailed);
        }
        let dil_len = u32::from_le_bytes(sig[..4].try_into().unwrap()) as usize;
        if sig.len() < 4 + dil_len + 64 {
            return Err(NexusError::IntegrityFailed);
        }
        let dil_sig = &sig[4..4 + dil_len];
        let ed_sig_bytes: [u8; 64] = sig[4 + dil_len..4 + dil_len + 64]
            .try_into()
            .map_err(|_| NexusError::IntegrityFailed)?;

        // Verify Dilithium5 signature.
        dilithium_verify(&self.signing_pq, msg, dil_sig)?;

        // Verify Ed25519 signature.
        let ed_sig = ed25519_dalek::Signature::from_bytes(&ed_sig_bytes);
        self.signing_classic
            .verify(msg, &ed_sig)
            .map_err(|_| NexusError::AuthFailed)?;

        Ok(())
    }

    /// BLAKE3 hash of all public key material.
    pub fn identity_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"NEXUS_IDENTITY_V1");
        hasher.update(self.signing_pq.as_bytes());
        hasher.update(self.signing_classic.as_bytes());
        hasher.update(&self.kem.to_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Serialize to bytes.
    /// Format: u32_le(dil_pk_len) || dil_pk || ed25519_pk(32) || kem_pk
    pub fn to_bytes(&self) -> Vec<u8> {
        let dil_bytes = self.signing_pq.as_bytes();
        let ed_bytes = self.signing_classic.as_bytes();
        let kem_bytes = self.kem.to_bytes();

        let dil_len = dil_bytes.len() as u32;
        let kem_len = kem_bytes.len() as u32;

        let mut out = Vec::with_capacity(4 + dil_bytes.len() + 32 + 4 + kem_bytes.len());
        out.extend_from_slice(&dil_len.to_le_bytes());
        out.extend_from_slice(dil_bytes);
        out.extend_from_slice(ed_bytes);
        out.extend_from_slice(&kem_len.to_le_bytes());
        out.extend_from_slice(&kem_bytes);
        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut pos = 0usize;

        // Dilithium public key.
        if bytes.len() < pos + 4 {
            return Err(NexusError::InvalidKey);
        }
        let dil_len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if bytes.len() < pos + dil_len {
            return Err(NexusError::InvalidKey);
        }
        let signing_pq = DilithiumPublicKey::from_bytes(&bytes[pos..pos + dil_len])
            .map_err(|_| NexusError::InvalidKey)?;
        pos += dil_len;

        // Ed25519 public key (32 bytes).
        if bytes.len() < pos + 32 {
            return Err(NexusError::InvalidKey);
        }
        let ed_bytes: [u8; 32] = bytes[pos..pos + 32].try_into().unwrap();
        let signing_classic =
            VerifyingKey::from_bytes(&ed_bytes).map_err(|_| NexusError::InvalidKey)?;
        pos += 32;

        // Hybrid KEM public key.
        if bytes.len() < pos + 4 {
            return Err(NexusError::InvalidKey);
        }
        let kem_len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if bytes.len() < pos + kem_len {
            return Err(NexusError::InvalidKey);
        }
        // HybridPublicKey::from_bytes expects the 4-byte kyber length prefix at the start.
        // The slice includes: [4-byte kyber_len][kyber_bytes][x25519_bytes]
        let kem = HybridPublicKey::from_bytes(&bytes[pos..pos + kem_len])?;
        pos += kem_len;

        // SECURITY: Validate that we consumed exactly all input bytes.
        // Silently ignoring trailing bytes could hide attacks or corrupted data.
        if pos != bytes.len() {
            return Err(NexusError::InvalidKey);
        }

        Ok(Self {
            signing_pq,
            signing_classic,
            kem,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_sign_verify() {
        let kp = IdentityKeyPair::generate();
        let pk = kp.public_key();
        let msg = b"NEXUS identity test";
        let sig = kp.sign(msg).unwrap();
        pk.verify(msg, &sig).unwrap();
    }

    #[test]
    fn test_identity_sign_wrong_msg_fails() {
        let kp = IdentityKeyPair::generate();
        let pk = kp.public_key();
        let sig = kp.sign(b"original").unwrap();
        assert!(pk.verify(b"tampered", &sig).is_err());
    }

    #[test]
    fn test_identity_public_key_serialization() {
        let kp = IdentityKeyPair::generate();
        let pk = kp.public_key();
        let bytes = pk.to_bytes();
        let pk2 = IdentityPublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(pk.identity_hash(), pk2.identity_hash());
    }

    #[test]
    fn test_identity_hash_stability() {
        let kp = IdentityKeyPair::generate();
        let h1 = kp.identity_hash();
        let h2 = kp.public_key().identity_hash();
        assert_eq!(h1, h2);
    }
}
