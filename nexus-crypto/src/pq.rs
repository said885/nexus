use pqcrypto_traits::sign::SignedMessage;

use crate::error::{NexusError, Result};

// ── Kyber1024 re-exports ──────────────────────────────────────────────────────

pub use pqcrypto_kyber::kyber1024::{
    Ciphertext as KyberCiphertext, PublicKey as KyberPublicKey, SecretKey as KyberSecretKey,
    SharedSecret as KyberSharedSecret,
};

pub use pqcrypto_kyber::kyber1024::{
    decapsulate as kyber_decapsulate_raw, encapsulate as kyber_encapsulate_raw,
    keypair as kyber_keypair_raw,
};

// ── Dilithium5 re-exports ─────────────────────────────────────────────────────

pub use pqcrypto_dilithium::dilithium5::{
    keypair as dilithium_keypair_raw, open as dilithium_open_raw, sign as dilithium_sign_raw,
    PublicKey as DilithiumPublicKey, SecretKey as DilithiumSecretKey,
};

// ── Kyber key pair wrapper ────────────────────────────────────────────────────

pub struct KyberKeyPair {
    pub public: KyberPublicKey,
    pub secret: KyberSecretKey,
}

impl KyberKeyPair {
    /// Generate a fresh Kyber1024 key pair.
    pub fn generate() -> Self {
        let (public, secret) = kyber_keypair_raw();
        Self { public, secret }
    }
}

// ── Dilithium key pair wrapper ────────────────────────────────────────────────

pub struct DilithiumKeyPair {
    pub public: DilithiumPublicKey,
    pub secret: DilithiumSecretKey,
}

impl DilithiumKeyPair {
    /// Generate a fresh Dilithium5 key pair.
    pub fn generate() -> Self {
        let (public, secret) = dilithium_keypair_raw();
        Self { public, secret }
    }
}

// ── Kyber KEM wrappers ────────────────────────────────────────────────────────

/// Encapsulate a shared secret under `pk`.
/// Returns `(shared_secret, ciphertext)`.
pub fn kyber_encap(pk: &KyberPublicKey) -> Result<(KyberSharedSecret, KyberCiphertext)> {
    let (ss, ct) = kyber_encapsulate_raw(pk);
    Ok((ss, ct))
}

/// Decapsulate the shared secret using `sk` and `ct`.
pub fn kyber_decap(sk: &KyberSecretKey, ct: &KyberCiphertext) -> Result<KyberSharedSecret> {
    let ss = kyber_decapsulate_raw(ct, sk);
    Ok(ss)
}

// ── Dilithium sign / verify ───────────────────────────────────────────────────

/// Sign `msg` with `sk`. Returns the detached signature bytes.
///
/// pqcrypto's `sign` returns a signed message (sig || msg). We strip the
/// message suffix so callers only store the signature.
pub fn dilithium_sign(sk: &DilithiumSecretKey, msg: &[u8]) -> Result<Vec<u8>> {
    let signed = dilithium_sign_raw(msg, sk);
    // The signed message format is: signature_bytes || original_message.
    // Signature length = signed.len() - msg.len()
    let signed_bytes = signed.as_bytes();
    if signed_bytes.len() < msg.len() {
        return Err(NexusError::CryptoError(
            "signed message shorter than original".into(),
        ));
    }
    let sig_len = signed_bytes.len() - msg.len();
    Ok(signed_bytes[..sig_len].to_vec())
}

/// Verify a detached Dilithium5 signature over `msg` with `pk`.
pub fn dilithium_verify(pk: &DilithiumPublicKey, msg: &[u8], sig: &[u8]) -> Result<()> {
    // Re-construct the signed message format that pqcrypto expects.
    let mut signed_bytes = sig.to_vec();
    signed_bytes.extend_from_slice(msg);

    let signed_msg = pqcrypto_dilithium::dilithium5::SignedMessage::from_bytes(&signed_bytes)
        .map_err(|_| NexusError::CryptoError("invalid signed message bytes".into()))?;

    dilithium_open_raw(&signed_msg, pk)
        .map_err(|_| NexusError::AuthFailed)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pqcrypto_traits::kem::SharedSecret;

    #[test]
    fn test_kyber_roundtrip() {
        let kp = KyberKeyPair::generate();
        let (ss_enc, ct) = kyber_encap(&kp.public).unwrap();
        let ss_dec = kyber_decap(&kp.secret, &ct).unwrap();
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_dilithium_sign_verify() {
        let kp = DilithiumKeyPair::generate();
        let msg = b"NEXUS test message";
        let sig = dilithium_sign(&kp.secret, msg).unwrap();
        dilithium_verify(&kp.public, msg, &sig).unwrap();
    }

    #[test]
    fn test_dilithium_wrong_msg_fails() {
        let kp = DilithiumKeyPair::generate();
        let msg = b"NEXUS test message";
        let sig = dilithium_sign(&kp.secret, msg).unwrap();
        assert!(dilithium_verify(&kp.public, b"tampered", &sig).is_err());
    }
}
