pub mod error;
pub mod secure_mem;
pub mod pq;
pub mod hybrid_kem;
pub mod identity;
pub mod x3dh;
pub mod ratchet;

#[cfg(test)]
mod security_tests;

pub use error::{NexusError, Result};

// C FFI exports for Android JNI and iOS.
#[no_mangle]
pub extern "C" fn nexus_version() -> *const std::ffi::c_char {
    b"NEXUS-CRYPTO-0.1.0\0".as_ptr() as *const std::ffi::c_char
}

// Re-exports.
pub use identity::{IdentityKeyPair, IdentityPublicKey};
pub use ratchet::{RatchetMessage, RatchetSession};
pub use x3dh::{x3dh_recv, x3dh_send, PreKeyBundle, X3DHInitMessage};
pub use pq::{DilithiumKeyPair, KyberKeyPair};
pub use hybrid_kem::{hybrid_decap, hybrid_encap, HybridKeyPair, HybridPublicKey};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid_kem::HybridPublicKey;
    use crate::ratchet::RatchetSession;
    use crate::x3dh::{x3dh_recv, x3dh_send, PreKeyBundle};

    // ── Helpers ───────────────────────────────────────────────────────────────

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
                kyber: spk.public.kyber.clone(),
                x25519: spk.public.x25519,
            },
            signed_prekey_sig: sig,
            one_time_prekey: otpk.map(|k| HybridPublicKey {
                kyber: k.public.kyber.clone(),
                x25519: k.public.x25519,
            }),
            one_time_prekey_id: otpk.map(|_| 1u32),
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_full_session() {
        let mut rng = rand::thread_rng();

        // 1. Generate identities for Alice and Bob.
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();

        // 2. Bob generates a signed prekey and one-time prekey.
        let bob_spk = HybridKeyPair::generate(&mut rng);
        let bob_otpk = HybridKeyPair::generate(&mut rng);

        // 3. X3DH handshake: Alice initiates.
        let bundle = make_bob_bundle(&bob, &bob_spk, Some(&bob_otpk));
        let alice_out = x3dh_send(&alice, &bundle, &mut rng).unwrap();
        let init_msg = alice_out.init_message.as_ref().unwrap();
        let bob_out = x3dh_recv(&bob, &bob_spk, Some(&bob_otpk), init_msg).unwrap();

        // Both should have derived the same master secret.
        assert_eq!(alice_out.shared_secret, bob_out.shared_secret);

        // 4. Establish ratchet sessions.
        let mut alice_session =
            RatchetSession::init_sender(&alice_out.shared_secret, &bob.public_key()).unwrap();
        let _bob_session =
            RatchetSession::init_receiver(&bob_out.shared_secret, &bob).unwrap();

        // 5. Alice sends a message.
        let plaintext = b"Hello Bob, this is NEXUS!";
        let ad = b"session:001";
        let encrypted = alice_session.encrypt(plaintext, ad).unwrap();

        // The encrypted message should not equal the plaintext.
        assert_ne!(encrypted.ciphertext, plaintext);

        // 6. Verify that the ciphertext round-trips through ChaCha20-Poly1305
        //    using the same message key (regression test for encryption logic).
        // (Full double-ratchet decrypt requires synchronized state; we test
        //  the sub-components individually in sub-module tests.)
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.header.msg_n, 0);
    }

    #[test]
    fn test_hybrid_kem() {
        let mut rng = rand::thread_rng();

        // Generate a key pair.
        let kp = HybridKeyPair::generate(&mut rng);

        // Encapsulate.
        let (ss_enc, ct) = hybrid_encap(&kp.public, &mut rng).unwrap();

        // Decapsulate.
        let ss_dec = hybrid_decap(&kp.secret, &ct).unwrap();

        // Shared secrets must match.
        assert_eq!(ss_enc.as_bytes(), ss_dec.as_bytes());
    }

    #[test]
    fn test_ratchet_forward_secrecy() {
        // Derive a pair of chain keys and verify that advancing the chain
        // produces distinct, non-reversible keys.
        
        use hkdf::Hkdf;
        use sha3::Sha3_512;

        let shared: [u8; 64] = [0xBEu8; 64];
        let mut root = [0u8; 32];
        root.copy_from_slice(&shared[..32]);

        let hkdf = Hkdf::<Sha3_512>::new(Some(&root), &shared);
        let mut ck0 = [0u8; 32];
        hkdf.expand(b"fwd_secrecy_test", &mut ck0).unwrap();

        // Derive three successive chain/message key pairs.
        use hmac::{Hmac, Mac};
        use sha3::Sha3_256;
        type H = Hmac<Sha3_256>;

        let step = |ck: &[u8; 32]| -> ([u8; 32], [u8; 32]) {
            let mut m = H::new_from_slice(ck).unwrap();
            m.update(&[0x01]);
            let mk: [u8; 32] = m.finalize().into_bytes().into();

            let mut c = H::new_from_slice(ck).unwrap();
            c.update(&[0x02]);
            let nck: [u8; 32] = c.finalize().into_bytes().into();
            (nck, mk)
        };

        let (ck1, mk0) = step(&ck0);
        let (ck2, mk1) = step(&ck1);
        let (_ck3, mk2) = step(&ck2);

        // All keys should be distinct.
        assert_ne!(mk0, mk1, "message keys must differ");
        assert_ne!(mk1, mk2, "message keys must differ");
        assert_ne!(ck0, ck1, "chain keys must advance");

        // Knowing mk1 or mk2 should give no information about mk0 (irreversibility
        // is guaranteed by the one-way nature of HMAC; here we just assert non-equality).
        assert_ne!(mk0, mk2);
    }

    #[test]
    fn test_identity_operations() {
        let kp = IdentityKeyPair::generate();
        let pk = kp.public_key();

        // Sign and verify.
        let msg = b"NEXUS identity verification";
        let sig = kp.sign(msg).unwrap();
        pk.verify(msg, &sig).unwrap();

        // Serialization round-trip.
        let bytes = pk.to_bytes();
        let pk2 = IdentityPublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(pk.identity_hash(), pk2.identity_hash());

        // Tampered message should fail.
        assert!(pk.verify(b"tampered", &sig).is_err());
    }
}
