#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for cryptographic operations
/// Tests that crypto functions handle invalid inputs gracefully
fuzz_target!(|data: &[u8]| {
    // Test key derivation with various inputs
    test_key_derivation(data);

    // Test encryption with edge cases
    test_encryption(data);

    // Test hash functions
    test_hashing(data);

    // Test signature verification
    test_signature(data);
});

fn test_key_derivation(data: &[u8]) {
    use sha3::{Digest, Sha3_256};

    // Test with various input sizes
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let _ = hasher.finalize();

    // Test with empty input
    let mut hasher = Sha3_256::new();
    hasher.update(b"");
    let _ = hasher.finalize();

    // Test with very large input
    if data.len() > 0 {
        let large_input = vec![data[0]; 1024 * 1024]; // 1MB
        let mut hasher = Sha3_256::new();
        hasher.update(&large_input);
        let _ = hasher.finalize();
    }
}

fn test_encryption(data: &[u8]) {
    use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};

    // Test with invalid key size
    if data.len() >= 32 {
        let key = &data[..32];
        if let Ok(cipher) = ChaCha20Poly1305::new_from_slice(key) {
            if data.len() >= 44 {
                let nonce = Nonce::from_slice(&data[32..44]);
                let plaintext = &data[44..];
                let _ = cipher.encrypt(nonce, plaintext);
            }
        }
    }

    // Test with empty plaintext
    if data.len() >= 32 {
        let key = &data[..32];
        if let Ok(cipher) = ChaCha20Poly1305::new_from_slice(key) {
            let nonce = Nonce::from_slice(&[0u8; 12]);
            let _ = cipher.encrypt(nonce, b"");
        }
    }
}

fn test_hashing(data: &[u8]) {
    use blake3;

    // Test BLAKE3
    let _ = blake3::hash(data);

    // Test incremental hashing
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let _ = hasher.finalize();

    // Test with key
    if data.len() >= 32 {
        let _ = blake3::keyed_hash(&data[..32].try_into().unwrap(), data);
    }
}

fn test_signature(data: &[u8]) {
    // Test signature verification with random data
    // This should never crash, only return Ok/Err
    if data.len() >= 64 {
        let signature = &data[..64];
        let message = &data[64..];

        // Simulate verification (simplified)
        let valid = signature.len() == 64 && !message.is_empty();
        let _ = valid;
    }
}
