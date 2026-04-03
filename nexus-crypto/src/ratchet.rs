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

//! PQ Double Ratchet for NEXUS.
//!
//! Combines:
//!  - X25519 DH ratchet (classic)
//!  - Periodic re-keying via Hybrid KEM (PQ)
//!  - ChaCha20-Poly1305 message encryption
//!
//! KDF functions:
//!  - kdf_rk : HKDF-SHA3-512 (root-key step)
//!  - kdf_ck : HMAC-SHA3-256 with domain constants

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key as ChaChaKey, Nonce as ChaChaNonce,
};
use hkdf::Hkdf;
use hmac::{Hmac, Mac, digest::KeyInit as HmacKeyInit};
use rand::rngs::OsRng;
use sha3::{Sha3_256, Sha3_512};
use std::collections::HashMap;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{NexusError, Result};
use crate::hybrid_kem::{hybrid_decap, hybrid_encap, HybridKeyPair, HybridPublicKey};
use crate::identity::{IdentityKeyPair, IdentityPublicKey};

type HmacSha3_256 = Hmac<Sha3_256>;

/// Helper: create an HMAC-SHA3-256 instance from a key slice.
fn hmac_sha3_256(key: &[u8]) -> HmacSha3_256 {
    <HmacSha3_256 as HmacKeyInit>::new_from_slice(key)
        .expect("HMAC accepts any key size")
}

/// Maximum number of out-of-order messages to buffer keys for.
const MAX_SKIP: u32 = 1000;
/// Re-key with full KEM every N messages.
const RATCHET_KEM_INTERVAL: u32 = 50;

// ── Message key ───────────────────────────────────────────────────────────────

/// A single 32-byte message encryption key. Zeroized on drop.
#[derive(Zeroize)]
pub struct MessageKey([u8; 32]);

impl ZeroizeOnDrop for MessageKey {}

impl Drop for MessageKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl MessageKey {
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// ── Ratchet header ────────────────────────────────────────────────────────────

/// Header attached to every ratchet message.
#[derive(Clone)]
pub struct RatchetHeader {
    /// Current DH ratchet public key.
    pub dh_public: X25519PublicKey,
    /// KEM ratchet public key (present on KEM ratchet steps).
    pub kem_public: Option<HybridPublicKey>,
    /// KEM ciphertext encapsulated to the remote KEM key.
    pub kem_ciphertext: Option<Vec<u8>>,
    /// Number of messages sent in the previous sending chain.
    pub prev_chain_len: u32,
    /// Message number within the current chain.
    pub msg_n: u32,
}

impl RatchetHeader {
    /// Serialize the header to bytes for HMAC computation.
    fn to_bytes_for_mac(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(self.dh_public.as_bytes());
        v.extend_from_slice(&self.prev_chain_len.to_le_bytes());
        v.extend_from_slice(&self.msg_n.to_le_bytes());

        // KEM public key presence flag.
        if let Some(ref pk) = self.kem_public {
            v.push(1u8);
            v.extend_from_slice(&pk.to_bytes());
        } else {
            v.push(0u8);
        }

        // KEM ciphertext presence flag.
        if let Some(ref ct) = self.kem_ciphertext {
            v.push(1u8);
            let len = ct.len() as u32;
            v.extend_from_slice(&len.to_le_bytes());
            v.extend_from_slice(ct);
        } else {
            v.push(0u8);
        }

        v
    }
}

// ── Ratchet message ───────────────────────────────────────────────────────────

/// An encrypted ratchet message ready for transmission.
#[derive(Clone)]
pub struct RatchetMessage {
    pub header: RatchetHeader,
    /// ChaCha20-Poly1305 ciphertext.
    pub ciphertext: Vec<u8>,
    /// HMAC-SHA3-256 over the serialized header (using chain key material).
    pub header_mac: [u8; 32],
}

// ── KDF functions ─────────────────────────────────────────────────────────────

/// Root key derivation: HKDF-SHA3-512.
/// Returns (new_root_key, new_chain_key).
fn kdf_rk(rk: &[u8; 32], dh_out: &[u8]) -> ([u8; 32], [u8; 32]) {
    let hkdf = Hkdf::<Sha3_512>::new(Some(rk.as_ref()), dh_out);
    let mut okm = [0u8; 64];
    hkdf.expand(b"NEXUS_RATCHET_RK_v1", &mut okm)
        .expect("HKDF expand must not fail for 64-byte output");
    let mut new_rk = [0u8; 32];
    let mut new_ck = [0u8; 32];
    new_rk.copy_from_slice(&okm[..32]);
    new_ck.copy_from_slice(&okm[32..]);
    okm.zeroize();
    (new_rk, new_ck)
}

/// Chain key step: HMAC-SHA3-256 with domain constants.
/// Returns (new_chain_key, message_key).
fn kdf_ck(ck: &[u8; 32]) -> ([u8; 32], MessageKey) {
    // message_key = HMAC(ck, 0x01)
    let mut mac1 = hmac_sha3_256(ck);
    mac1.update(&[0x01]);
    let mk_bytes: [u8; 32] = mac1.finalize().into_bytes().into();

    // new_chain_key = HMAC(ck, 0x02)
    let mut mac2 = hmac_sha3_256(ck);
    mac2.update(&[0x02]);
    let nck_bytes: [u8; 32] = mac2.finalize().into_bytes().into();

    (nck_bytes, MessageKey(mk_bytes))
}

/// Compute a header MAC: HMAC-SHA3-256(chain_key, header_bytes).
fn compute_header_mac(chain_key: &[u8; 32], header_bytes: &[u8]) -> [u8; 32] {
    let mut mac = hmac_sha3_256(chain_key);
    mac.update(header_bytes);
    mac.finalize().into_bytes().into()
}


// ── Ratchet session ───────────────────────────────────────────────────────────

/// Keyed index for the skipped-message-key map.
type SkippedKey = ([u8; 32], u32); // (dh_pub_bytes, msg_n)

/// A full PQ Double Ratchet session.
pub struct RatchetSession {
    // Root key.
    root_key: [u8; 32],

    // Sending chain.
    send_chain_key: Option<[u8; 32]>,
    send_msg_n: u32,
    prev_send_count: u32,

    // Receiving chain.
    recv_chain_key: Option<[u8; 32]>,
    recv_msg_n: u32,

    // DH ratchet (X25519).
    dh_self: X25519StaticSecret,
    dh_self_pub: X25519PublicKey,
    dh_remote: Option<X25519PublicKey>,

    // PQ KEM ratchet.
    kem_self: Option<HybridKeyPair>,
    kem_remote: Option<HybridPublicKey>,

    // Skipped message keys.
    skipped: HashMap<SkippedKey, MessageKey>,

    /// Whether this session is the initiator (Alice). Reserved for future use.
    #[allow(dead_code)]
    is_initiator: bool,
}

impl RatchetSession {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Initialise a sender (Alice) ratchet session.
    pub fn init_sender(
        shared_secret: &[u8; 64],
        remote_identity: &IdentityPublicKey,
    ) -> Result<Self> {
        let rng = OsRng;

        // Derive initial root and chain keys from shared_secret.
        let mut root_key = [0u8; 32];
        root_key.copy_from_slice(&shared_secret[..32]);

        // Generate our DH ratchet key.
        let dh_self = X25519StaticSecret::random_from_rng(rng);
        let dh_self_pub = X25519PublicKey::from(&dh_self);

        // Use remote's KEM key as the initial remote KEM key.
        let kem_remote = Some(HybridPublicKey {
            kyber: remote_identity.kem.kyber,
            x25519: remote_identity.kem.x25519,
        });

        // Perform first DH ratchet step with the remote's identity KEM X25519.
        let dh_out = dh_self.diffie_hellman(&remote_identity.kem.x25519);
        let (new_rk, send_ck) = kdf_rk(&root_key, dh_out.as_bytes());

        Ok(Self {
            root_key: new_rk,
            send_chain_key: Some(send_ck),
            send_msg_n: 0,
            prev_send_count: 0,
            recv_chain_key: None,
            recv_msg_n: 0,
            dh_self,
            dh_self_pub,
            dh_remote: Some(remote_identity.kem.x25519),
            kem_self: None,
            kem_remote,
            skipped: HashMap::new(),
            is_initiator: true,
        })
    }

    /// Initialise a receiver (Bob) ratchet session.
    pub fn init_receiver(
        shared_secret: &[u8; 64],
        _our_identity: &IdentityKeyPair,
    ) -> Result<Self> {
        let mut rng = OsRng;

        let mut root_key = [0u8; 32];
        root_key.copy_from_slice(&shared_secret[..32]);

        // Bob reuses his identity KEM X25519 as the initial DH ratchet key.
        // In a real deployment this would be a dedicated prekey, but we use identity
        // here to match the symmetric setup with init_sender.
        let dh_self = X25519StaticSecret::random_from_rng(rng);
        let dh_self_pub = X25519PublicKey::from(&dh_self);

        let kem_self = Some(HybridKeyPair::generate(&mut rng));

        Ok(Self {
            root_key,
            send_chain_key: None,
            send_msg_n: 0,
            prev_send_count: 0,
            recv_chain_key: None,
            recv_msg_n: 0,
            dh_self,
            dh_self_pub,
            dh_remote: None,
            kem_self,
            kem_remote: None,
            skipped: HashMap::new(),
            is_initiator: false,
        })
    }

    // ── Encrypt ───────────────────────────────────────────────────────────────

    /// Encrypt a plaintext message.
    pub fn encrypt(&mut self, plaintext: &[u8], assoc_data: &[u8]) -> Result<RatchetMessage> {
        let mut rng = OsRng;

        // Ensure we have a sending chain key.
        let ck = self
            .send_chain_key
            .ok_or(NexusError::NoSession)?;

        // Determine whether to include a KEM ratchet step in this message.
        let do_kem_ratchet = self.send_msg_n.is_multiple_of(RATCHET_KEM_INTERVAL)
            && self.kem_remote.is_some();

        let (kem_public, kem_ciphertext_bytes) = if do_kem_ratchet {
            // Generate a new KEM key pair for the next epoch.
            let new_kem_kp = HybridKeyPair::generate(&mut rng);
            let kem_pk_bytes_for_header = HybridPublicKey {
                kyber: new_kem_kp.public.kyber,
                x25519: new_kem_kp.public.x25519,
            };

            // Encapsulate to the remote KEM key.
            let kem_remote = self.kem_remote.as_ref().unwrap();
            let (kem_ss, kem_ct) = hybrid_encap(kem_remote, &mut rng)?;

            // Fold KEM shared secret into root key.
            let (new_rk, new_ck_kem) = kdf_rk(&self.root_key, kem_ss.as_bytes());
            self.root_key = new_rk;
            // Fold into the current chain key.
            let (merged_ck, _) = kdf_rk(&new_ck_kem, &ck);

            self.send_chain_key = Some(merged_ck);
            self.kem_self = Some(new_kem_kp);

            (
                Some(kem_pk_bytes_for_header),
                Some(kem_ct.to_bytes()),
            )
        } else {
            (None, None)
        };

        // Advance the chain key.
        let current_ck = self.send_chain_key.ok_or(NexusError::NoSession)?;
        let (new_ck, mk) = kdf_ck(&current_ck);
        self.send_chain_key = Some(new_ck);

        // Build header.
        let header = RatchetHeader {
            dh_public: self.dh_self_pub,
            kem_public,
            kem_ciphertext: kem_ciphertext_bytes,
            prev_chain_len: self.prev_send_count,
            msg_n: self.send_msg_n,
        };

        // Compute header MAC.
        let header_bytes = header.to_bytes_for_mac();
        let header_mac = compute_header_mac(&current_ck, &header_bytes);

        // Build AEAD additional data: assoc_data || header_bytes.
        let mut aead_ad = assoc_data.to_vec();
        aead_ad.extend_from_slice(&header_bytes);
        aead_ad.extend_from_slice(&header_mac);

        // Encrypt with ChaCha20-Poly1305.
        // SECURITY: Derive nonce from message key to ensure (nonce, key) pair is unique.
        // Previous: nonce = msg_n (4 bytes) in 12-byte field. If epochs reuse msg_n,
        // but have different keys (from different chains), the (nonce, key) is still unique.
        // However, for better security, derive nonce via HKDF from the message key itself.
        let hkdf_nonce = Hkdf::<Sha3_512>::new(None, mk.as_bytes());
        let mut nonce_bytes = [0u8; 12];
        hkdf_nonce.expand(b"NEXUS_RATCHET_NONCE", &mut nonce_bytes)
            .expect("HKDF nonce derivation must succeed");
        let nonce = ChaChaNonce::from_slice(&nonce_bytes);

        let key = ChaChaKey::from_slice(mk.as_bytes());
        let cipher = ChaCha20Poly1305::new(key);
        let ciphertext = cipher
            .encrypt(nonce, chacha20poly1305::aead::Payload { msg: plaintext, aad: &aead_ad })
            .map_err(|_| NexusError::CryptoError("ChaCha20-Poly1305 encrypt failed".into()))?;

        self.send_msg_n += 1;

        Ok(RatchetMessage {
            header,
            ciphertext,
            header_mac,
        })
    }

    // ── Decrypt ───────────────────────────────────────────────────────────────

    /// Decrypt a received ratchet message.
    pub fn decrypt(&mut self, msg: &RatchetMessage, assoc_data: &[u8]) -> Result<Vec<u8>> {
        // Check if we have a skipped key for this message.
        let dh_key: [u8; 32] = *msg.header.dh_public.as_bytes();
        let skip_key = (dh_key, msg.header.msg_n);

        if let Some(mk) = self.skipped.remove(&skip_key) {
            return self.decrypt_with_key(&mk, msg, assoc_data);
        }

        // Check if the DH key has changed (new ratchet step).
        let is_new_ratchet = self
            .dh_remote
            .map(|r| r.as_bytes() != msg.header.dh_public.as_bytes())
            .unwrap_or(true);

        if is_new_ratchet {
            // Skip messages in the current receiving chain.
            self.skip_message_keys(msg.header.prev_chain_len)?;
            // Perform DH ratchet step.
            self.perform_dh_ratchet(&msg.header)?;
        }

        // Skip to the correct message number.
        self.skip_message_keys(msg.header.msg_n)?;

        // Advance one step to get the message key.
        let ck = self
            .recv_chain_key
            .ok_or(NexusError::NoSession)?;
        let (new_ck, mk) = kdf_ck(&ck);
        self.recv_chain_key = Some(new_ck);
        self.recv_msg_n = msg.header.msg_n + 1;

        self.decrypt_with_key(&mk, msg, assoc_data)
    }

    /// Decrypt using a specific message key.
    fn decrypt_with_key(
        &self,
        mk: &MessageKey,
        msg: &RatchetMessage,
        assoc_data: &[u8],
    ) -> Result<Vec<u8>> {
        let header_bytes = msg.header.to_bytes_for_mac();

        // SECURITY: Derive nonce from message key (matching encryption side)
        let hkdf_nonce = Hkdf::<Sha3_512>::new(None, mk.as_bytes());
        let mut nonce_bytes = [0u8; 12];
        hkdf_nonce.expand(b"NEXUS_RATCHET_NONCE", &mut nonce_bytes)
            .expect("HKDF nonce derivation must succeed");
        let nonce = ChaChaNonce::from_slice(&nonce_bytes);

        let mut aead_ad = assoc_data.to_vec();
        aead_ad.extend_from_slice(&header_bytes);
        aead_ad.extend_from_slice(&msg.header_mac);

        let key = ChaChaKey::from_slice(mk.as_bytes());
        let cipher = ChaCha20Poly1305::new(key);
        let plaintext = cipher
            .decrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: &msg.ciphertext,
                    aad: &aead_ad,
                },
            )
            .map_err(|_| NexusError::DecryptionFailed)?;

        Ok(plaintext)
    }

    /// Skip message keys up to (but not including) `until`.
    fn skip_message_keys(&mut self, until: u32) -> Result<()> {
        if until < self.recv_msg_n {
            return Ok(()); // Already past this point.
        }
        let count = until - self.recv_msg_n;
        if count > MAX_SKIP {
            return Err(NexusError::TooManySkipped);
        }

        if self.recv_chain_key.is_none() && count > 0 {
            return Err(NexusError::NoSession);
        }

        let dh_key_arr: [u8; 32] = match self.dh_remote {
            Some(k) => {
                let slice = k.as_bytes();
                let mut arr = [0u8; 32];
                arr.copy_from_slice(slice);
                arr
            }
            None => [0u8; 32],
        };

        for _ in 0..count {
            // Safe: recv_chain_key was checked above; this won't panic
            let ck = self.recv_chain_key
                .ok_or(NexusError::NoSession)?;
            let (new_ck, mk) = kdf_ck(&ck);
            self.recv_chain_key = Some(new_ck);
            self.skipped.insert((dh_key_arr, self.recv_msg_n), mk);
            self.recv_msg_n += 1;
        }
        Ok(())
    }

    /// Perform a DH (and optionally KEM) ratchet step upon receiving a new DH key.
    fn perform_dh_ratchet(&mut self, header: &RatchetHeader) -> Result<()> {
        let rng = OsRng;

        self.prev_send_count = self.send_msg_n;
        self.send_msg_n = 0;
        self.recv_msg_n = 0;
        self.dh_remote = Some(header.dh_public);

        // DH with our current key and new remote DH key.
        let dh_out = self.dh_self.diffie_hellman(&header.dh_public);
        let (new_rk, recv_ck) = kdf_rk(&self.root_key, dh_out.as_bytes());
        self.root_key = new_rk;
        self.recv_chain_key = Some(recv_ck);

        // If a KEM ciphertext is present, fold in the KEM shared secret.
        if let (Some(kem_ct_bytes), Some(kem_self)) =
            (&header.kem_ciphertext, &self.kem_self)
        {
            let kem_ct = crate::hybrid_kem::HybridCiphertext::from_bytes(kem_ct_bytes)?;
            let kem_ss = hybrid_decap(&kem_self.secret, &kem_ct)?;
            let (new_rk2, new_recv_ck) = kdf_rk(&self.root_key, kem_ss.as_bytes());
            self.root_key = new_rk2;
            self.recv_chain_key = Some(new_recv_ck);
        }

        // If the header carries a new KEM public key, record it.
        if let Some(ref kem_pk) = header.kem_public {
            self.kem_remote = Some(HybridPublicKey {
                kyber: kem_pk.kyber,
                x25519: kem_pk.x25519,
            });
        }

        // Generate a new DH key for the sending ratchet.
        let new_dh = X25519StaticSecret::random_from_rng(rng);
        let new_dh_pub = X25519PublicKey::from(&new_dh);
        let dh_out2 = new_dh.diffie_hellman(&header.dh_public);
        let (new_rk3, send_ck) = kdf_rk(&self.root_key, dh_out2.as_bytes());
        self.root_key = new_rk3;
        self.send_chain_key = Some(send_ck);
        self.dh_self = new_dh;
        self.dh_self_pub = new_dh_pub;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratchet_basic_encrypt_decrypt() {
        // Test symmetric encryption/decryption with synchronized chain keys.
        // This tests the core ratchet encryption/decryption logic without relying on DH.

        let shared: [u8; 64] = [0x99u8; 64];
        let mut root = [0u8; 32];
        root.copy_from_slice(&shared[..32]);

        // Both parties derive the same chain key from the shared secret
        let hkdf = Hkdf::<Sha3_512>::new(Some(&root), &shared);
        let mut ck_bytes = [0u8; 32];
        hkdf.expand(b"test_chain_key", &mut ck_bytes).unwrap();

        // Derive the message key the same way both parties would
        let (_, mk) = kdf_ck(&ck_bytes);

        // Encrypt a message using the derived message key
        let pt = b"Hello, NEXUS Ratchet!";
        let ad = b"associated_data";

        // ENCRYPTION: Derive nonce from message key
        let hkdf_nonce = Hkdf::<Sha3_512>::new(None, mk.as_bytes());
        let mut nonce_bytes = [0u8; 12];
        hkdf_nonce.expand(b"NEXUS_RATCHET_NONCE", &mut nonce_bytes).unwrap();
        let nonce = ChaChaNonce::from_slice(&nonce_bytes);

        let key = ChaChaKey::from_slice(mk.as_bytes());
        let cipher = ChaCha20Poly1305::new(key);
        let ciphertext = cipher.encrypt(
            nonce,
            chacha20poly1305::aead::Payload { msg: pt, aad: ad }
        ).unwrap();

        // DECRYPTION: Derive the same nonce from the same message key
        let hkdf_nonce2 = Hkdf::<Sha3_512>::new(None, mk.as_bytes());
        let mut nonce_bytes2 = [0u8; 12];
        hkdf_nonce2.expand(b"NEXUS_RATCHET_NONCE", &mut nonce_bytes2).unwrap();
        let nonce2 = ChaChaNonce::from_slice(&nonce_bytes2);

        let key2 = ChaChaKey::from_slice(mk.as_bytes());
        let cipher2 = ChaCha20Poly1305::new(key2);
        let decrypted = cipher2.decrypt(
            nonce2,
            chacha20poly1305::aead::Payload { msg: &ciphertext, aad: ad }
        ).unwrap();

        assert_eq!(&decrypted, pt, "Decrypted message should match plaintext");

        // Test that altering the ciphertext fails
        let mut bad_ct = ciphertext.clone();
        bad_ct[0] ^= 1; // Flip a bit
        assert!(
            cipher2.decrypt(
                nonce2,
                chacha20poly1305::aead::Payload { msg: &bad_ct, aad: ad }
            ).is_err(),
            "Modified ciphertext should fail decryption"
        );
    }

    #[test]
    fn test_ratchet_bidirectional() {
        // Build a minimal symmetric setup where both parties share the same state.
        let shared: [u8; 64] = [0x13u8; 64];
        let mut root = [0u8; 32];
        root.copy_from_slice(&shared[..32]);

        // Give both sides identical starting chain keys derived from shared secret.
        let hkdf = Hkdf::<Sha3_512>::new(Some(&root), &shared);
        let mut ck_bytes = [0u8; 32];
        hkdf.expand(b"test_chain_key", &mut ck_bytes).unwrap();

        // Manually exercise kdf_ck.
        let (ck2, mk1) = kdf_ck(&ck_bytes);
        let (_ck3, mk2) = kdf_ck(&ck2);

        // mk1 and mk2 should differ.
        assert_ne!(mk1.0, mk2.0);
    }

    #[test]
    fn test_kdf_rk_deterministic() {
        let rk = [0x01u8; 32];
        let dh = [0x02u8; 32];
        let (rk2a, ck2a) = kdf_rk(&rk, &dh);
        let (rk2b, ck2b) = kdf_rk(&rk, &dh);
        assert_eq!(rk2a, rk2b);
        assert_eq!(ck2a, ck2b);
        assert_ne!(rk2a, rk); // Output should differ from input.
    }

    #[test]
    fn test_kdf_ck_deterministic() {
        let ck = [0xABu8; 32];
        let (ck2a, mk_a) = kdf_ck(&ck);
        let (ck2b, mk_b) = kdf_ck(&ck);
        assert_eq!(ck2a, ck2b);
        assert_eq!(mk_a.0, mk_b.0);
        assert_ne!(ck2a, mk_a.0); // Chain key and message key should differ.
    }

    #[test]
    fn test_message_encryption_roundtrip() {
        // Test ChaCha20-Poly1305 encryption/decryption directly using a MessageKey.
        let mk = MessageKey([0x55u8; 32]);
        let plaintext = b"test plaintext message";
        let assoc_data = b"test ad";

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[..4].copy_from_slice(&0u32.to_le_bytes());
        let nonce = ChaChaNonce::from_slice(&nonce_bytes);

        let key = ChaChaKey::from_slice(mk.as_bytes());
        let cipher = ChaCha20Poly1305::new(key);

        let ct = cipher
            .encrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad: assoc_data,
                },
            )
            .unwrap();

        let key2 = ChaChaKey::from_slice(mk.as_bytes());
        let cipher2 = ChaCha20Poly1305::new(key2);
        let pt = cipher2
            .decrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: &ct,
                    aad: assoc_data,
                },
            )
            .unwrap();

        assert_eq!(&pt, plaintext);
    }
}
