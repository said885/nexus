import XCTest
@testable import NexusMessenger

final class SecureEnclaveManagerTests: XCTestCase {

    var manager: SecureEnclaveManager!

    override func setUp() {
        super.setUp()
        manager = SecureEnclaveManager()
    }

    override func tearDown() {
        // Clean up any test keys
        try? manager.deleteKey(tag: "com.nexus.test.signing")
        try? manager.deleteKey(tag: "com.nexus.test.agreement")
        manager = nil
        super.tearDown()
    }

    // MARK: - Availability

    func testSecureEnclaveAvailability() {
        // This test documents whether SE is available on the test device
        let isAvailable = manager.isAvailable
        #if targetEnvironment(simulator)
        XCTAssertFalse(isAvailable, "Secure Enclave should not be available on simulator")
        #else
        // On real devices, this depends on the hardware
        XCTAssertTrue(isAvailable, "Secure Enclave should be available on real devices")
        #endif
    }

    // MARK: - Key Generation

    func testSigningKeyGeneration() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let key = try manager.generateSigningKey(tag: "com.nexus.test.signing", requireBiometry: false)
        XCTAssertNotNil(key, "Signing key should be generated")
    }

    func testKeyAgreementKeyGeneration() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let key = try manager.generateKeyAgreementKey(tag: "com.nexus.test.agreement", requireBiometry: false)
        XCTAssertNotNil(key, "Key agreement key should be generated")
    }

    func testKeyPersistence() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let tag = "com.nexus.test.signing"

        // Generate key
        let key1 = try manager.generateSigningKey(tag: tag, requireBiometry: false)
        XCTAssertNotNil(key1)

        // Load same key
        let key2 = try manager.loadSigningKey(tag: tag)
        XCTAssertNotNil(key2)

        // Keys should be the same
        let key1Data = SecKeyCopyExternalRepresentation(key1, nil) as! Data
        let key2Data = SecKeyCopyExternalRepresentation(key2, nil) as! Data
        XCTAssertEqual(key1Data, key2Data, "Loaded key should match generated key")
    }

    // MARK: - Key Wrapping

    func testKeyWrapUnwrap() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let secretData = Data(repeating: 0x42, count: 32)

        let wrapped = try manager.wrapKey(secretData)
        XCTAssertNotEqual(wrapped, secretData, "Wrapped key should differ from original")

        let unwrapped = try manager.unwrapKey(wrapped)
        XCTAssertEqual(unwrapped, secretData, "Unwrapped key should match original")
    }

    func testKeyWrapProducesDifferentCiphertext() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let secretData = Data(repeating: 0x42, count: 32)

        let wrapped1 = try manager.wrapKey(secretData)
        let wrapped2 = try manager.wrapKey(secretData)

        // Due to random nonces, wrapped versions should differ
        XCTAssertNotEqual(wrapped1, wrapped2, "Multiple wraps should produce different results")
    }

    // MARK: - Key Deletion

    func testKeyDeletion() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let tag = "com.nexus.test.signing"

        // Generate and then delete
        _ = try manager.generateSigningKey(tag: tag, requireBiometry: false)
        try manager.deleteKey(tag: tag)

        // Loading should fail after deletion
        XCTAssertThrowsError(try manager.loadSigningKey(tag: tag)) { error in
            // Expected error
        }
    }

    // MARK: - Key Listing

    func testListNexusKeys() throws {
        guard manager.isAvailable else {
            throw XCTSkip("Secure Enclave not available on this device")
        }

        let initialCount = manager.listNexusKeys().count

        let tag = "com.nexus.test.signing"
        _ = try manager.generateSigningKey(tag: tag, requireBiometry: false)

        let afterCount = manager.listNexusKeys().count
        XCTAssertGreaterThan(afterCount, initialCount, "Key count should increase after generation")
    }
}
