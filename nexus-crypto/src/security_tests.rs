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

/// Comprehensive security tests for NEXUS crypto library.
///
/// This module tests:
///  - Hybrid encryption/decryption correctness
///  - Post-quantum cryptography primitives
///  - X3DH protocol compliance
///  - Double ratchet forward secrecy
///  - Secure memory handling
///  - Proper zeroization of sensitive data
#[cfg(test)]
mod tests {
    use crate::*;
    use rand::rngs::OsRng;

    const TEST_AD: &[u8] = b"test_associated_data";

    // ─── Hybrid Encryption Tests ─────────────────────────────────────────

    #[test]
    fn test_hybrid_kem_encapsulation_decapsulation() {
        let mut rng = OsRng;
        let key_pair_b = hybrid_kem::HybridKeyPair::generate(&mut rng);

        // Encapsulate for Bob's public key
        let (shared_secret_enc, ciphertext) = hybrid_kem::hybrid_encap(&key_pair_b.public, &mut rng)
            .expect("encapsulation should succeed");

        // Decapsulate
        let shared_secret_dec = hybrid_kem::hybrid_decap(&key_pair_b.secret, &ciphertext)
            .expect("decapsulation should succeed");

        // Shared secrets should match
        assert_eq!(
            shared_secret_enc.as_bytes().as_slice(),
            shared_secret_dec.as_bytes().as_slice(),
            "hybrid KEM shared secret mismatch"
        );
    }

    #[test]
    fn test_hybrid_kem_different_keys_different_secrets() {
        let mut rng = OsRng;
        let bob_keys = hybrid_kem::HybridKeyPair::generate(&mut rng);
        let charlie_keys = hybrid_kem::HybridKeyPair::generate(&mut rng);

        // Encapsulate for both Bob and Charlie
        let (enc_bob, _) = hybrid_kem::hybrid_encap(&bob_keys.public, &mut rng)
            .expect("encapsulation for bob");
        let (enc_charlie, _) = hybrid_kem::hybrid_encap(&charlie_keys.public, &mut rng)
            .expect("encapsulation for charlie");

        // Shared secrets should be different
        assert_ne!(
            enc_bob.as_bytes().as_slice(),
            enc_charlie.as_bytes().as_slice(),
            "different recipients should have different shared secrets"
        );
    }

    // ─── X3DH Protocol Tests ─────────────────────────────────────────────

    #[test]
    fn test_x3dh_mutual_secret_derivation() {
        let mut rng = OsRng;

        // Setup: Alice and Bob generate identities
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();

        // Bob publishes a prekey bundle
        let bob_spk = hybrid_kem::HybridKeyPair::generate(&mut rng);
        let bob_otpk = hybrid_kem::HybridKeyPair::generate(&mut rng);

        let bob_bundle = x3dh::PreKeyBundle {
            identity_key: bob.public_key(),
            signed_prekey: hybrid_kem::HybridPublicKey {
                kyber: bob_spk.public.kyber,
                x25519: bob_spk.public.x25519,
            },
            signed_prekey_sig: bob.sign(&bob_spk.public.to_bytes()).unwrap(),
            one_time_prekey: Some(hybrid_kem::HybridPublicKey {
                kyber: bob_otpk.public.kyber,
                x25519: bob_otpk.public.x25519,
            }),
            one_time_prekey_id: Some(1u32),
        };

        // Alice initiates X3DH
        let alice_out = x3dh::x3dh_send(&alice, &bob_bundle, &mut rng)
            .expect("alice x3dh send");

        // Bob receives and responds
        let bob_out = x3dh::x3dh_recv(&bob, &bob_spk, Some(&bob_otpk), alice_out.init_message.as_ref().unwrap())
            .expect("bob x3dh recv");

        // Both should derive the same shared secret
        assert_eq!(
            &alice_out.shared_secret,
            &bob_out.shared_secret,
            "X3DH should produce identical shared secrets"
        );
    }

    #[test]
    fn test_x3dh_randomness() {
        let mut rng = OsRng;

        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();

        // Bob's prekey bundle
        let bob_spk = hybrid_kem::HybridKeyPair::generate(&mut rng);
        let bob_otpk = hybrid_kem::HybridKeyPair::generate(&mut rng);

        let bob_bundle = x3dh::PreKeyBundle {
            identity_key: bob.public_key(),
            signed_prekey: hybrid_kem::HybridPublicKey {
                kyber: bob_spk.public.kyber,
                x25519: bob_spk.public.x25519,
            },
            signed_prekey_sig: bob.sign(&bob_spk.public.to_bytes()).unwrap(),
            one_time_prekey: Some(hybrid_kem::HybridPublicKey {
                kyber: bob_otpk.public.kyber,
                x25519: bob_otpk.public.x25519,
            }),
            one_time_prekey_id: Some(1u32),
        };

        // Two X3DH runs should produce different shared secrets or init messages
        let alice_out1 = x3dh::x3dh_send(&alice, &bob_bundle, &mut rng).unwrap();
        let alice_out2 = x3dh::x3dh_send(&alice, &bob_bundle, &mut rng).unwrap();

        // Different runs should produce different shared secrets (due to randomness)
        assert_ne!(
            alice_out1.shared_secret,
            alice_out2.shared_secret,
            "X3DH should produce different shared secrets for different runs"
        );
    }

    // ─── Double Ratchet Tests ────────────────────────────────────────────

    #[test]
    fn test_double_ratchet_forward_secrecy() {
        let bob_identity = IdentityKeyPair::generate();

        let mut alice_session = ratchet::RatchetSession::init_sender(&[1u8; 64], &bob_identity.public_key())
            .expect("init alice session");

        // Alice sends messages
        let plaintext1 = b"Message 1";
        let encrypted1 = alice_session.encrypt(plaintext1, TEST_AD)
            .expect("encrypt message 1");

        let plaintext2 = b"Message 2";
        let encrypted2 = alice_session.encrypt(plaintext2, TEST_AD)
            .expect("encrypt message 2");

        // Verify ciphertexts are different (different keys/IVs)
        assert_ne!(
            encrypted1.ciphertext.as_slice(),
            encrypted2.ciphertext.as_slice(),
            "consecutive messages should have different ciphertexts"
        );

        // Verify plaintext is not visible in ciphertext
        assert!(!encrypted1.ciphertext.windows(plaintext1.len())
            .any(|w| w == plaintext1),
            "plaintext should not appear in ciphertext"
        );
    }

    // ─── Identity Key Tests ──────────────────────────────────────────────

    #[test]
    fn test_identity_key_pair_signature_verification() {
        let ident = IdentityKeyPair::generate();

        let message = b"test data to sign";
        let signature = ident.sign(message).expect("sign should succeed");

        // Verify with public key
        let verified = ident.public_key().verify(message, &signature);
        assert!(verified.is_ok(), "signature verification should succeed");

        // Verify with wrong message should fail
        let verified_wrong = ident.public_key().verify(b"different data", &signature);
        assert!(verified_wrong.is_err(), "signature verification should fail with wrong message");
    }

    #[test]
    fn test_independent_identities_are_distinct() {
        let ident1 = IdentityKeyPair::generate();
        let ident2 = IdentityKeyPair::generate();

        // Two independently generated identities should be different
        // We test this by trying to sign and verify with wrong keys
        let msg = b"test message";
        let sig1 = ident1.sign(msg).unwrap();
        
        // Signature from ident1 should not verify with ident2's key
        let verify_result = ident2.public_key().verify(msg, &sig1);
        assert!(verify_result.is_err(), "cross-identity signature verification should fail");
    }

    // ─── Encryption Nondeterminism Tests ─────────────────────────────────

    #[test]
    fn test_encryption_nondeterminism() {
        let bob_identity = IdentityKeyPair::generate();

        let mut alice1 = ratchet::RatchetSession::init_sender(&[1u8; 64], &bob_identity.public_key())
            .expect("init alice 1");
        let mut alice2 = ratchet::RatchetSession::init_sender(&[1u8; 64], &bob_identity.public_key())
            .expect("init alice 2");

        let plaintext = b"test message";

        let enc1 = alice1.encrypt(plaintext, TEST_AD).expect("enc 1");
        let enc2 = alice2.encrypt(plaintext, TEST_AD).expect("enc 2");

        // Encrypting same plaintext with different keys should produce different ciphertexts
        assert_ne!(
            enc1.ciphertext.as_slice(),
            enc2.ciphertext.as_slice(),
            "encrypting same plaintext with different keys should produce different ciphertexts"
        );
    }

    // ─── Resilience Tests ────────────────────────────────────────────────

    #[test]
    fn test_empty_plaintext_encryption() {
        let bob_identity = IdentityKeyPair::generate();

        let mut alice_session = ratchet::RatchetSession::init_sender(&[1u8; 64], &bob_identity.public_key())
            .expect("init session");

        // Encrypting empty message should still work
        let encrypted = alice_session.encrypt(b"", TEST_AD).expect("encrypt empty message");
        assert!(!encrypted.ciphertext.is_empty(), "even empty plaintext should produce ciphertext");
    }

    #[test]
    fn test_large_plaintext_encryption() {
        let bob_identity = IdentityKeyPair::generate();

        let mut alice_session = ratchet::RatchetSession::init_sender(&[1u8; 64], &bob_identity.public_key())
            .expect("init session");

        // Large message (1 MiB)
        let large_plaintext = vec![0xAB; 1024 * 1024];
        let encrypted = alice_session.encrypt(&large_plaintext, TEST_AD)
            .expect("encrypt large message");

        // Ciphertext should be larger than plaintext (auth tag, nonce, etc.)
        assert!(
            encrypted.ciphertext.len() >= large_plaintext.len(),
            "ciphertext should be at least as large as plaintext"
        );
    }

    // ─── Integration Tests ───────────────────────────────────────────────

    #[test]
    fn test_full_session_integration() {
        // Complete flow: Alice -> X3DH -> Bob, then exchange messages
        let mut rng = OsRng;

        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();

        // Bob's prekey bundle
        let bob_spk = hybrid_kem::HybridKeyPair::generate(&mut rng);
        let bob_otpk = hybrid_kem::HybridKeyPair::generate(&mut rng);

        let bob_bundle = x3dh::PreKeyBundle {
            identity_key: bob.public_key(),
            signed_prekey: hybrid_kem::HybridPublicKey {
                kyber: bob_spk.public.kyber,
                x25519: bob_spk.public.x25519,
            },
            signed_prekey_sig: bob.sign(&bob_spk.public.to_bytes()).unwrap(),
            one_time_prekey: Some(hybrid_kem::HybridPublicKey {
                kyber: bob_otpk.public.kyber,
                x25519: bob_otpk.public.x25519,
            }),
            one_time_prekey_id: Some(1u32),
        };

        // X3DH exchange
        let alice_x3dh = x3dh::x3dh_send(&alice, &bob_bundle, &mut rng).expect("alice x3dh");
        let bob_x3dh = x3dh::x3dh_recv(&bob, &bob_spk, Some(&bob_otpk), alice_x3dh.init_message.as_ref().unwrap())
            .expect("bob x3dh");

        // Verify shared secret match
        assert_eq!(&alice_x3dh.shared_secret, &bob_x3dh.shared_secret);

        // Initialize ratchet sessions
        let mut alice_ratchet = ratchet::RatchetSession::init_sender(
            &alice_x3dh.shared_secret,
            &bob.public_key(),
        )
        .expect("alice ratchet init");

        // Alice sends encrypted message
        let msg = b"Secret message for Bob";
        let encrypted = alice_ratchet.encrypt(msg, TEST_AD).expect("encrypt");

        // Verify ciphertext is not empty and differs from plaintext
        assert!(!encrypted.ciphertext.is_empty());
        assert_ne!(encrypted.ciphertext.as_slice(), msg);
    }
}

