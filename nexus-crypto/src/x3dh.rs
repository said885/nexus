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

//! PQ-X3DH key agreement protocol for NEXUS.
//!
//! Combines post-quantum KEM operations with classical Diffie-Hellman
//! to establish a shared master secret between two parties.
//!
//! Master secret derivation:
//!   HKDF-SHA3-512(
//!       ikm  = 0xFF*32 || kem_ik_ss || kem_spk_ss || kem_otpk_ss || eph_dh_ss,
//!       salt = zeros(64),
//!       info = b"NEXUS_X3DH_v1"
//!   )
//!   where kem_otpk_ss is zero-padded when no one-time prekey is used.

use hkdf::Hkdf;
use rand_core::CryptoRngCore;
use sha3::Sha3_512;

use crate::error::{NexusError, Result};
use crate::hybrid_kem::{hybrid_decap, hybrid_encap, HybridCiphertext, HybridKeyPair, HybridPublicKey, HybridSharedSecret};
use crate::identity::{IdentityKeyPair, IdentityPublicKey};

// ── Prekey bundle (Bob's published material) ──────────────────────────────────

/// Bob's prekey bundle, published to the server so Alice can initiate.
pub struct PreKeyBundle {
    /// Bob's long-term identity key.
    pub identity_key: IdentityPublicKey,
    /// Bob's medium-term signed prekey.
    pub signed_prekey: HybridPublicKey,
    /// Signature over `signed_prekey.to_bytes()` by Bob's identity key.
    pub signed_prekey_sig: Vec<u8>,
    /// Optional one-time prekey (removed from server after use).
    pub one_time_prekey: Option<HybridPublicKey>,
    /// Identifier for the one-time prekey.
    pub one_time_prekey_id: Option<u32>,
}

// ── X3DH messages ─────────────────────────────────────────────────────────────

/// Alice's initial message sent to Bob.
pub struct X3DHInitMessage {
    /// Alice's identity public key.
    pub identity_key: IdentityPublicKey,
    /// Alice's ephemeral hybrid public key.
    pub ephemeral_key: HybridPublicKey,
    /// Which one-time prekey was used (if any).
    pub one_time_prekey_id: Option<u32>,
    /// KEM ciphertext encapsulated to Bob's identity KEM key.
    pub kem_ciphertext_ik: Vec<u8>,
    /// KEM ciphertext encapsulated to Bob's signed prekey.
    pub kem_ciphertext_spk: Vec<u8>,
    /// KEM ciphertext encapsulated to Bob's one-time prekey (if used).
    pub kem_ciphertext_otpk: Option<Vec<u8>>,
}

// ── X3DH output ───────────────────────────────────────────────────────────────

/// Result of the X3DH handshake.
pub struct X3DHOutput {
    /// The 64-byte combined master secret.
    pub shared_secret: [u8; 64],
    /// The init message (Some only for the initiator / Alice).
    pub init_message: Option<X3DHInitMessage>,
}

// ── Ciphertext serialization helpers ─────────────────────────────────────────

/// Serialize a HybridCiphertext to bytes for transmission.
fn serialize_ct(ct: &HybridCiphertext) -> Vec<u8> {
    use pqcrypto_traits::kem::Ciphertext as KemCt;
    let kyber_bytes = ct.kyber_ct.as_bytes();
    let kyber_len = kyber_bytes.len() as u32;
    let mut out = Vec::with_capacity(4 + kyber_bytes.len() + 32);
    out.extend_from_slice(&kyber_len.to_le_bytes());
    out.extend_from_slice(kyber_bytes);
    out.extend_from_slice(ct.x25519_ephemeral.as_bytes());
    out
}

/// Deserialize a HybridCiphertext from bytes.
fn deserialize_ct(bytes: &[u8]) -> Result<HybridCiphertext> {
    use pqcrypto_kyber::kyber1024::Ciphertext as KyberCt;
    use pqcrypto_traits::kem::Ciphertext as KemCt;
    use x25519_dalek::PublicKey as X25519PublicKey;

    if bytes.len() < 4 {
        return Err(NexusError::CryptoError("ciphertext too short".into()));
    }
    let kyber_len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + kyber_len + 32 {
        return Err(NexusError::CryptoError("ciphertext truncated".into()));
    }
    let kyber_ct = KyberCt::from_bytes(&bytes[4..4 + kyber_len])
        .map_err(|_| NexusError::CryptoError("invalid Kyber ciphertext".into()))?;
    let x25519_bytes: [u8; 32] = bytes[4 + kyber_len..4 + kyber_len + 32]
        .try_into()
        .map_err(|_| NexusError::CryptoError("invalid x25519 ephemeral".into()))?;
    let x25519_ephemeral = X25519PublicKey::from(x25519_bytes);

    Ok(HybridCiphertext { kyber_ct, x25519_ephemeral })
}

// ── KDF ───────────────────────────────────────────────────────────────────────

/// Derive the master secret from all KEM shared secrets.
fn derive_master_secret(
    kem_ik_ss: &[u8],
    kem_spk_ss: &[u8],
    kem_otpk_ss: Option<&[u8]>,
    eph_dh_ss: &[u8],
) -> Result<[u8; 64]> {
    use zeroize::Zeroize;

    // IKM = 0xFF*32 || kem_ik_ss || kem_spk_ss || kem_otpk_ss_or_zeros || eph_dh_ss
    let mut ikm = Vec::new();
    ikm.extend_from_slice(&[0xFF; 32]);
    ikm.extend_from_slice(kem_ik_ss);
    ikm.extend_from_slice(kem_spk_ss);
    match kem_otpk_ss {
        Some(ss) => ikm.extend_from_slice(ss),
        None => ikm.extend_from_slice(&[0u8; 64]), // zero-fill when no OTPK
    }
    ikm.extend_from_slice(eph_dh_ss);

    let salt = [0u8; 64];
    let hkdf = Hkdf::<Sha3_512>::new(Some(&salt), &ikm);
    let mut okm = [0u8; 64];
    hkdf.expand(b"NEXUS_X3DH_v1", &mut okm)
        .map_err(|_| NexusError::CryptoError("X3DH HKDF failed".into()))?;

    // CRITICAL: Zeroize IKM after HKDF to prevent secrets from lingering in memory
    ikm.zeroize();

    Ok(okm)
}

// ── Alice: initiator ──────────────────────────────────────────────────────────

/// Alice performs X3DH and produces the master secret + init message.
pub fn x3dh_send(
    alice_identity: &IdentityKeyPair,
    bob_bundle: &PreKeyBundle,
    rng: &mut impl CryptoRngCore,
) -> Result<X3DHOutput> {
    // 1. Verify Bob's signed prekey signature.
    let spk_bytes = bob_bundle.signed_prekey.to_bytes();
    bob_bundle
        .identity_key
        .verify(&spk_bytes, &bob_bundle.signed_prekey_sig)
        .map_err(|_| NexusError::CryptoError("Bob's signed prekey signature invalid".into()))?;

    // 2. Generate Alice's ephemeral KEM key pair.
    let alice_eph = HybridKeyPair::generate(rng);

    // 3. KEM to Bob's identity key.
    let (kem_ik_ss, kem_ik_ct) = hybrid_encap(&bob_bundle.identity_key.kem, rng)?;

    // 4. KEM to Bob's signed prekey.
    let (kem_spk_ss, kem_spk_ct) = hybrid_encap(&bob_bundle.signed_prekey, rng)?;

    // 5. KEM to Bob's one-time prekey (optional).
    let (kem_otpk_ss_opt, kem_otpk_ct_opt, otpk_id) =
        if let Some(ref otpk) = bob_bundle.one_time_prekey {
            let (ss, ct) = hybrid_encap(otpk, rng)?;
            (Some(ss), Some(ct), bob_bundle.one_time_prekey_id)
        } else {
            (None, None, None)
        };

    // 6. Ephemeral DH between Alice's ephemeral key and Bob's signed prekey (X25519).
    let eph_dh_ss = alice_eph
        .secret
        .x25519
        .diffie_hellman(&bob_bundle.signed_prekey.x25519);

    // 7. Derive master secret.
    let master = derive_master_secret(
        kem_ik_ss.as_bytes(),
        kem_spk_ss.as_bytes(),
        kem_otpk_ss_opt.as_ref().map(|s| s.as_bytes().as_slice()),
        eph_dh_ss.as_bytes(),
    )?;

    // 8. Serialize ciphertexts.
    let kem_ciphertext_ik = serialize_ct(&kem_ik_ct);
    let kem_ciphertext_spk = serialize_ct(&kem_spk_ct);
    let kem_ciphertext_otpk = kem_otpk_ct_opt.as_ref().map(serialize_ct);

    let init_message = X3DHInitMessage {
        identity_key: alice_identity.public_key(),
        ephemeral_key: HybridPublicKey {
            kyber: alice_eph.public.kyber,
            x25519: alice_eph.public.x25519,
        },
        one_time_prekey_id: otpk_id,
        kem_ciphertext_ik,
        kem_ciphertext_spk,
        kem_ciphertext_otpk,
    };

    Ok(X3DHOutput {
        shared_secret: master,
        init_message: Some(init_message),
    })
}

// ── Bob: receiver ─────────────────────────────────────────────────────────────

/// Bob processes Alice's init message and derives the same master secret.
pub fn x3dh_recv(
    bob_identity: &IdentityKeyPair,
    bob_signed_prekey: &HybridKeyPair,
    bob_one_time_prekey: Option<&HybridKeyPair>,
    msg: &X3DHInitMessage,
) -> Result<X3DHOutput> {
    // 1. Decapsulate KEM to Bob's identity key.
    let ct_ik = deserialize_ct(&msg.kem_ciphertext_ik)?;
    let kem_ik_ss = hybrid_decap(&bob_identity.kem.secret, &ct_ik)?;

    // 2. Decapsulate KEM to Bob's signed prekey.
    let ct_spk = deserialize_ct(&msg.kem_ciphertext_spk)?;
    let kem_spk_ss = hybrid_decap(&bob_signed_prekey.secret, &ct_spk)?;

    // 3. Decapsulate KEM to Bob's one-time prekey (optional).
    let kem_otpk_ss_opt: Option<HybridSharedSecret> =
        if let Some(ct_bytes) = &msg.kem_ciphertext_otpk {
            let ct = deserialize_ct(ct_bytes)?;
            let otpk = bob_one_time_prekey
                .ok_or_else(|| NexusError::CryptoError("expected one-time prekey".into()))?;
            Some(hybrid_decap(&otpk.secret, &ct)?)
        } else {
            None
        };

    // 4. Ephemeral DH: Bob's signed prekey (static) × Alice's ephemeral X25519.
    let eph_dh_ss = bob_signed_prekey
        .secret
        .x25519
        .diffie_hellman(&msg.ephemeral_key.x25519);

    // 5. Derive master secret.
    let master = derive_master_secret(
        kem_ik_ss.as_bytes(),
        kem_spk_ss.as_bytes(),
        kem_otpk_ss_opt.as_ref().map(|s| s.as_bytes().as_slice()),
        eph_dh_ss.as_bytes(),
    )?;

    Ok(X3DHOutput {
        shared_secret: master,
        init_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::IdentityKeyPair;

    fn make_bob_bundle(
        bob: &IdentityKeyPair,
        spk: &HybridKeyPair,
        otpk: Option<&HybridKeyPair>,
    ) -> PreKeyBundle {
        let spk_bytes = spk.public.to_bytes();
        let sig = bob.sign(&spk_bytes).unwrap();
        PreKeyBundle {
            identity_key: bob.public_key(),
            signed_prekey: HybridPublicKey {
                kyber: spk.public.kyber,
                x25519: spk.public.x25519,
            },
            signed_prekey_sig: sig,
            one_time_prekey: otpk.map(|k| HybridPublicKey {
                kyber: k.public.kyber,
                x25519: k.public.x25519,
            }),
            one_time_prekey_id: otpk.map(|_| 1u32),
        }
    }

    #[test]
    fn test_x3dh_without_otpk() {
        let mut rng = rand::thread_rng();
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();
        let bob_spk = HybridKeyPair::generate(&mut rng);

        let bundle = make_bob_bundle(&bob, &bob_spk, None);
        let alice_out = x3dh_send(&alice, &bundle, &mut rng).unwrap();
        let init_msg = alice_out.init_message.as_ref().unwrap();
        let bob_out = x3dh_recv(&bob, &bob_spk, None, init_msg).unwrap();

        assert_eq!(alice_out.shared_secret, bob_out.shared_secret);
    }

    #[test]
    fn test_x3dh_with_otpk() {
        let mut rng = rand::thread_rng();
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();
        let bob_spk = HybridKeyPair::generate(&mut rng);
        let bob_otpk = HybridKeyPair::generate(&mut rng);

        let bundle = make_bob_bundle(&bob, &bob_spk, Some(&bob_otpk));
        let alice_out = x3dh_send(&alice, &bundle, &mut rng).unwrap();
        let init_msg = alice_out.init_message.as_ref().unwrap();
        let bob_out = x3dh_recv(&bob, &bob_spk, Some(&bob_otpk), init_msg).unwrap();

        assert_eq!(alice_out.shared_secret, bob_out.shared_secret);
    }
}
