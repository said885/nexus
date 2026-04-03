import XCTest
@testable import NexusMessenger

final class NexusCryptoTests: XCTestCase {

    // MARK: - Identity Generation

    func testIdentityGeneration() throws {
        let crypto = NexusCrypto()
        let identity = try crypto.generateIdentity()

        XCTAssertFalse(identity.kyberPublicKey.isEmpty, "Kyber public key should not be empty")
        XCTAssertFalse(identity.x25519PublicKey.isEmpty, "X25519 public key should not be empty")
        XCTAssertFalse(identity.signingPublicKey.isEmpty, "Signing public key should not be empty")
        XCTAssertFalse(identity.identityHash.isEmpty, "Identity hash should not be empty")
        XCTAssertEqual(identity.identityHash.count, 64, "Identity hash should be 64 hex chars (SHA3-256)")
    }

    func testIdentityHashDeterministic() throws {
        let crypto = NexusCrypto()
        let identity1 = try crypto.generateIdentity()
        let identity2 = try crypto.generateIdentity()

        // Different identities should have different hashes
        XCTAssertNotEqual(identity1.identityHash, identity2.identityHash,
                         "Different identities should have different hashes")
    }

    // MARK: - HKDF Key Derivation

    func testHKDFDerivation() {
        let crypto = NexusCrypto()
        let inputKey = Data(repeating: 0x42, count: 32)
        let salt = Data(repeating: 0x01, count: 16)
        let info = Data("test-info".utf8)

        let derived1 = crypto.hkdf(inputKey: inputKey, salt: salt, info: info, outputLength: 32)
        let derived2 = crypto.hkdf(inputKey: inputKey, salt: salt, info: info, outputLength: 32)

        XCTAssertEqual(derived1, derived2, "HKDF should be deterministic")
        XCTAssertEqual(derived1.count, 32, "Output length should match requested")
    }

    func testHKDFDifferentInputs() {
        let crypto = NexusCrypto()
        let inputKey1 = Data(repeating: 0x42, count: 32)
        let inputKey2 = Data(repeating: 0x43, count: 32)
        let salt = Data(repeating: 0x01, count: 16)
        let info = Data("test-info".utf8)

        let derived1 = crypto.hkdf(inputKey: inputKey1, salt: salt, info: info, outputLength: 32)
        let derived2 = crypto.hkdf(inputKey: inputKey2, salt: salt, info: info, outputLength: 32)

        XCTAssertNotEqual(derived1, derived2, "Different inputs should produce different outputs")
    }

    // MARK: - AES-GCM Encryption

    func testAESGCMEncryptDecrypt() throws {
        let crypto = NexusCrypto()
        let key = Data(repeating: 0x42, count: 32)
        let plaintext = Data("Hello, NEXUS!".utf8)

        let encrypted = try crypto.aesgcmEncrypt(key: key, plaintext: plaintext)
        XCTAssertNotEqual(encrypted.ciphertext, plaintext, "Ciphertext should differ from plaintext")
        XCTAssertFalse(encrypted.nonce.isEmpty, "Nonce should not be empty")
        XCTAssertFalse(encrypted.tag.isEmpty, "Tag should not be empty")

        let decrypted = try crypto.aesgcmDecrypt(key: key, nonce: encrypted.nonce, ciphertext: encrypted.ciphertext, tag: encrypted.tag)
        XCTAssertEqual(decrypted, plaintext, "Decrypted text should match original")
    }

    func testAESGCMDifferentNonces() throws {
        let crypto = NexusCrypto()
        let key = Data(repeating: 0x42, count: 32)
        let plaintext = Data("Hello, NEXUS!".utf8)

        let encrypted1 = try crypto.aesgcmEncrypt(key: key, plaintext: plaintext)
        let encrypted2 = try crypto.aesgcmEncrypt(key: key, plaintext: plaintext)

        XCTAssertNotEqual(encrypted1.nonce, encrypted2.nonce, "Nonces should be random")
        XCTAssertNotEqual(encrypted1.ciphertext, encrypted2.ciphertext, "Ciphertexts should differ with different nonces")
    }

    // MARK: - Double Ratchet

    func testRatchetStateInitialization() throws {
        let crypto = NexusCrypto()
        let sharedSecret = Data(repeating: 0x42, count: 32)

        let state = crypto.initRatchetSender(sharedSecret: sharedSecret)
        XCTAssertFalse(state.rootKey.isEmpty, "Root key should not be empty")
        XCTAssertFalse(state.sendChainKey.isEmpty, "Send chain key should not be empty")
        XCTAssertEqual(state.sendCounter, 0, "Send counter should start at 0")
        XCTAssertEqual(state.recvCounter, 0, "Receive counter should start at 0")
    }

    func testRatchetEncryptDecrypt() throws {
        let crypto = NexusCrypto()
        let sharedSecret = Data(repeating: 0x42, count: 32)

        var senderState = crypto.initRatchetSender(sharedSecret: sharedSecret)
        var receiverState = crypto.initRatchetReceiver(sharedSecret: sharedSecret)

        let message = Data("Secret message".utf8)
        let encrypted = try crypto.ratchetEncrypt(state: &senderState, plaintext: message)

        XCTAssertNotEqual(encrypted.ciphertext, message, "Ciphertext should differ from plaintext")

        let decrypted = try crypto.ratchetDecrypt(state: &receiverState, encrypted: encrypted)
        XCTAssertEqual(decrypted, message, "Decrypted message should match original")
    }

    func testRatchetForwardSecrecy() throws {
        let crypto = NexusCrypto()
        let sharedSecret = Data(repeating: 0x42, count: 32)

        var senderState = crypto.initRatchetSender(sharedSecret: sharedSecret)

        let message1 = Data("Message 1".utf8)
        let message2 = Data("Message 2".utf8)

        let encrypted1 = try crypto.ratchetEncrypt(state: &senderState, plaintext: message1)
        let encrypted2 = try crypto.ratchetEncrypt(state: &senderState, plaintext: message2)

        XCTAssertNotEqual(encrypted1.ciphertext, encrypted2.ciphertext,
                         "Different messages should have different ciphertexts")
        XCTAssertNotEqual(encrypted1.msgN, encrypted2.msgN,
                         "Message numbers should increment")
    }

    // MARK: - Secure Wipe

    func testSecureWipe() {
        let crypto = NexusCrypto()
        var sensitiveData = Data(repeating: 0xFF, count: 64)

        crypto.secureWipe(&sensitiveData)

        // After secure wipe, data should be zeroed
        let allZeros = sensitiveData.allSatisfy { $0 == 0 }
        XCTAssertTrue(allZeros, "Secure wipe should zero out all bytes")
    }

    // MARK: - Error Handling

    func testDecryptWithWrongKey() throws {
        let crypto = NexusCrypto()
        let key1 = Data(repeating: 0x42, count: 32)
        let key2 = Data(repeating: 0x43, count: 32)
        let plaintext = Data("Secret".utf8)

        let encrypted = try crypto.aesgcmEncrypt(key: key1, plaintext: plaintext)

        XCTAssertThrowsError(try crypto.aesgcmDecrypt(key: key2, nonce: encrypted.nonce, ciphertext: encrypted.ciphertext, tag: encrypted.tag)) { error in
            XCTAssertTrue(error is NexusCryptoError, "Should throw NexusCryptoError")
        }
    }

    func testDecryptWithCorruptedCiphertext() throws {
        let crypto = NexusCrypto()
        let key = Data(repeating: 0x42, count: 32)
        let plaintext = Data("Secret".utf8)

        let encrypted = try crypto.aesgcmEncrypt(key: key, plaintext: plaintext)
        var corruptedCiphertext = encrypted.ciphertext
        corruptedCiphertext[0] = corruptedCiphertext[0] ^ 0xFF

        XCTAssertThrowsError(try crypto.aesgcmDecrypt(key: key, nonce: encrypted.nonce, ciphertext: corruptedCiphertext, tag: encrypted.tag)) { error in
            XCTAssertTrue(error is NexusCryptoError, "Should throw NexusCryptoError for corrupted ciphertext")
        }
    }
}
