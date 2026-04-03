//! Hybrid KEM: Kyber1024 + X25519, combined via HKDF-SHA3-512.
//!
//! The combined shared secret is derived as:
//!   HKDF-SHA3-512(ikm = kyber_ss || x25519_ss,
//!                 salt = b"NEXUS_HYBRID_KEM",
//!                 info = b"nexus_hybrid_v1")

use hkdf::Hkdf;
use pqcrypto_traits::kem::{PublicKey as KemPublicKey, SharedSecret as KemSharedSecret};
use rand_core::CryptoRngCore;
use sha3::Sha3_512;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519SecretKey};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{NexusError, Result};
use crate::pq::{
    kyber_decap, kyber_encap, KyberCiphertext, KyberKeyPair, KyberPublicKey, KyberSecretKey,
};

// ── Ciphertext ────────────────────────────────────────────────────────────────

/// Ciphertext produced during hybrid encapsulation.
pub struct HybridCiphertext {
    /// Kyber ciphertext.
    pub kyber_ct: KyberCiphertext,
    /// X25519 ephemeral public key (sent alongside the ciphertext).
    pub x25519_ephemeral: X25519PublicKey,
}

impl HybridCiphertext {
    /// Serialize to bytes: kyber_len (u32 LE) || kyber_ct || x25519_ephemeral (32 bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        use pqcrypto_traits::kem::Ciphertext as KemCt;
        let kyber_bytes = self.kyber_ct.as_bytes();
        let kyber_len = kyber_bytes.len() as u32;
        let mut out = Vec::with_capacity(4 + kyber_bytes.len() + 32);
        out.extend_from_slice(&kyber_len.to_le_bytes());
        out.extend_from_slice(kyber_bytes);
        out.extend_from_slice(self.x25519_ephemeral.as_bytes());
        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        use pqcrypto_kyber::kyber1024::Ciphertext as KyberCt;
        use pqcrypto_traits::kem::Ciphertext as KemCt;

        if bytes.len() < 4 {
            return Err(NexusError::InvalidKey);
        }
        let kyber_len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        if bytes.len() < 4 + kyber_len + 32 {
            return Err(NexusError::InvalidKey);
        }
        let kyber_ct = KyberCt::from_bytes(&bytes[4..4 + kyber_len])
            .map_err(|_| NexusError::InvalidKey)?;
        let x25519_bytes: [u8; 32] = bytes[4 + kyber_len..4 + kyber_len + 32]
            .try_into()
            .map_err(|_| NexusError::InvalidKey)?;
        let x25519_ephemeral = X25519PublicKey::from(x25519_bytes);

        Ok(Self {
            kyber_ct,
            x25519_ephemeral,
        })
    }
}

// ── Public key ────────────────────────────────────────────────────────────────

/// Hybrid public key combining Kyber1024 and X25519.
#[derive(Clone)]
pub struct HybridPublicKey {
    pub kyber: KyberPublicKey,
    pub x25519: X25519PublicKey,
}

impl HybridPublicKey {
    /// Serialize to bytes: kyber_pk_len (u32 LE) || kyber_pk || x25519_pk (32 bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let kyber_bytes = self.kyber.as_bytes();
        let mut out = Vec::with_capacity(4 + kyber_bytes.len() + 32);
        let len = kyber_bytes.len() as u32;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(kyber_bytes);
        out.extend_from_slice(self.x25519.as_bytes());
        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(NexusError::InvalidKey);
        }
        let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        if bytes.len() < 4 + len + 32 {
            return Err(NexusError::InvalidKey);
        }
        let kyber_bytes = &bytes[4..4 + len];
        let x25519_bytes: [u8; 32] = bytes[4 + len..4 + len + 32]
            .try_into()
            .map_err(|_| NexusError::InvalidKey)?;

        let kyber = KyberPublicKey::from_bytes(kyber_bytes)
            .map_err(|_| NexusError::InvalidKey)?;
        let x25519 = X25519PublicKey::from(x25519_bytes);
        Ok(Self { kyber, x25519 })
    }
}

// ── Secret key ────────────────────────────────────────────────────────────────

/// Hybrid secret key. Both halves are zeroized on drop.
pub struct HybridSecretKey {
    pub(crate) kyber: KyberSecretKey,
    pub(crate) x25519: X25519SecretKey,
}

impl Drop for HybridSecretKey {
    fn drop(&mut self) {
        // X25519SecretKey implements ZeroizeOnDrop automatically.

        // KyberSecretKey doesn't implement Zeroize, so we manually zero its bytes.
        // SAFETY: Kyber secret key is a fixed-size type, and we can safely overwrite
        // its memory via the SecretKey trait's as_bytes() method.
        use pqcrypto_traits::kem::SecretKey as KemSecretKey;

        // Get the byte representation and zero it out
        let kyber_bytes: &[u8] = self.kyber.as_bytes();
        let len = kyber_bytes.len();

        // Cast to mutable pointer and zero via volatile writes
        let ptr = kyber_bytes.as_ptr() as *mut u8;

        // Use volatile write to ensure compiler doesn't optimize away the zeroing
        for i in 0..len {
            unsafe {
                std::ptr::write_volatile(ptr.add(i), 0);
            }
        }

        // Use a fence to prevent reordering of memory operations
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }
}

// ── Key pair ──────────────────────────────────────────────────────────────────

/// A hybrid (Kyber + X25519) key pair.
pub struct HybridKeyPair {
    pub public: HybridPublicKey,
    pub(crate) secret: HybridSecretKey,
}

impl HybridKeyPair {
    /// Generate a fresh hybrid key pair.
    pub fn generate(rng: &mut impl CryptoRngCore) -> Self {
        let kyber_kp = KyberKeyPair::generate();
        let x25519_secret = X25519SecretKey::random_from_rng(rng);
        let x25519_public = X25519PublicKey::from(&x25519_secret);

        Self {
            public: HybridPublicKey {
                kyber: kyber_kp.public,
                x25519: x25519_public,
            },
            secret: HybridSecretKey {
                kyber: kyber_kp.secret,
                x25519: x25519_secret,
            },
        }
    }
}

// ── Shared secret ─────────────────────────────────────────────────────────────

/// Combined 64-byte shared secret from both KEM mechanisms.
#[derive(Zeroize)]
pub struct HybridSharedSecret(pub [u8; 64]);

impl ZeroizeOnDrop for HybridSharedSecret {}

impl Drop for HybridSharedSecret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl HybridSharedSecret {
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

// ── KDF helper ────────────────────────────────────────────────────────────────

/// Combine kyber_ss and x25519_ss into the 64-byte hybrid shared secret.
fn combine_secrets(kyber_ss: &[u8], x25519_ss: &[u8]) -> Result<HybridSharedSecret> {
    let mut ikm = Vec::with_capacity(kyber_ss.len() + x25519_ss.len());
    ikm.extend_from_slice(kyber_ss);
    ikm.extend_from_slice(x25519_ss);

    let hkdf = Hkdf::<Sha3_512>::new(Some(b"NEXUS_HYBRID_KEM"), &ikm);
    let mut okm = [0u8; 64];
    hkdf.expand(b"nexus_hybrid_v1", &mut okm)
        .map_err(|_| NexusError::CryptoError("HKDF expand failed".into()))?;

    // Zeroize the IKM buffer.
    ikm.zeroize();

    Ok(HybridSharedSecret(okm))
}

// ── Encapsulate ───────────────────────────────────────────────────────────────

/// Encapsulate a hybrid shared secret under `pk`.
/// Returns `(shared_secret, ciphertext)`.
pub fn hybrid_encap(
    pk: &HybridPublicKey,
    rng: &mut impl CryptoRngCore,
) -> Result<(HybridSharedSecret, HybridCiphertext)> {
    // Kyber encapsulation.
    let (kyber_ss, kyber_ct) = kyber_encap(&pk.kyber)?;

    // X25519: generate ephemeral key pair and perform DH.
    let eph_secret = X25519SecretKey::random_from_rng(rng);
    let eph_public = X25519PublicKey::from(&eph_secret);
    let x25519_ss_point = eph_secret.diffie_hellman(&pk.x25519);

    let kyber_ss_bytes = KemSharedSecret::as_bytes(&kyber_ss);
    let combined = combine_secrets(kyber_ss_bytes, x25519_ss_point.as_bytes())?;

    Ok((
        combined,
        HybridCiphertext {
            kyber_ct,
            x25519_ephemeral: eph_public,
        },
    ))
}

// ── Decapsulate ───────────────────────────────────────────────────────────────

/// Decapsulate a hybrid shared secret using `sk` and `ct`.
pub fn hybrid_decap(sk: &HybridSecretKey, ct: &HybridCiphertext) -> Result<HybridSharedSecret> {
    // Kyber decapsulation.
    let kyber_ss = kyber_decap(&sk.kyber, &ct.kyber_ct)?;

    // X25519 DH.
    let x25519_ss_point = sk.x25519.diffie_hellman(&ct.x25519_ephemeral);

    let kyber_ss_bytes = KemSharedSecret::as_bytes(&kyber_ss);
    combine_secrets(kyber_ss_bytes, x25519_ss_point.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_kem_roundtrip() {
        let mut rng = rand::thread_rng();
        let kp = HybridKeyPair::generate(&mut rng);
        let (ss_enc, ct) = hybrid_encap(&kp.public, &mut rng).unwrap();
        let ss_dec = hybrid_decap(&kp.secret, &ct).unwrap();
        assert_eq!(ss_enc.0, ss_dec.0);
    }

    #[test]
    fn test_hybrid_public_key_serialization() {
        let mut rng = rand::thread_rng();
        let kp = HybridKeyPair::generate(&mut rng);
        let bytes = kp.public.to_bytes();
        let pk2 = HybridPublicKey::from_bytes(&bytes).unwrap();
        // Verify round-trip by re-serializing.
        assert_eq!(bytes, pk2.to_bytes());
    }

    #[test]
    fn test_wrong_secret_key_gives_different_secret() {
        let mut rng = rand::thread_rng();
        let kp1 = HybridKeyPair::generate(&mut rng);
        let kp2 = HybridKeyPair::generate(&mut rng);
        let (ss1, ct) = hybrid_encap(&kp1.public, &mut rng).unwrap();
        // Decapsulate with wrong key -- should not panic, but give different result.
        let ss_wrong = hybrid_decap(&kp2.secret, &ct).unwrap();
        assert_ne!(ss1.0, ss_wrong.0);
    }
}
