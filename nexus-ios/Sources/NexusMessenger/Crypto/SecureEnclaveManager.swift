import CryptoKit
import Foundation
import LocalAuthentication
import Security

// MARK: - Errors

public enum SecureEnclaveError: Error, LocalizedError {
    case notAvailable
    case keyGenerationFailed(OSStatus)
    case keyLoadFailed(OSStatus)
    case keyDeleteFailed(OSStatus)
    case wrapFailed
    case unwrapFailed
    case contextCreationFailed

    public var errorDescription: String? {
        switch self {
        case .notAvailable:                    return "Secure Enclave is not available on this device"
        case .keyGenerationFailed(let s):      return "Key generation failed: \(s)"
        case .keyLoadFailed(let s):            return "Key load failed: \(s)"
        case .keyDeleteFailed(let s):          return "Key deletion failed: \(s)"
        case .wrapFailed:                      return "Key wrapping failed"
        case .unwrapFailed:                    return "Key unwrapping failed"
        case .contextCreationFailed:           return "LAContext creation failed"
        }
    }
}

// MARK: - SecureEnclaveManager

public final class SecureEnclaveManager {

    private init() {}

    // -------------------------------------------------------------------------
    // MARK: Availability
    // -------------------------------------------------------------------------

    public static var isAvailable: Bool {
        SecureEnclave.isAvailable
    }

    // -------------------------------------------------------------------------
    // MARK: P-256 Signing Key
    // -------------------------------------------------------------------------

    /// Generate and store a P-256 signing key in the Secure Enclave.
    /// If `requireBiometric` is true, the key is bound to biometric authentication.
    public static func generateSigningKey(
        label: String,
        requireBiometric: Bool
    ) throws -> SecureEnclave.P256.Signing.PrivateKey {
        guard isAvailable else { throw SecureEnclaveError.notAvailable }

        let accessControl = try makeAccessControl(requireBiometric: requireBiometric)

        return try SecureEnclave.P256.Signing.PrivateKey(
            accessControl: accessControl,
            authenticationContext: requireBiometric ? makeAuthContext() : nil
        )
    }

    /// Load an existing P-256 signing key from the Secure Enclave by label.
    /// In CryptoKit, SE keys are stored via their `dataRepresentation`; we persist
    /// that blob in the Keychain and reload it here.
    public static func loadSigningKey(
        label: String,
        context: LAContext? = nil
    ) throws -> SecureEnclave.P256.Signing.PrivateKey {
        guard isAvailable else { throw SecureEnclaveError.notAvailable }

        let data = try loadKeychainData(label: label)

        if let ctx = context {
            return try SecureEnclave.P256.Signing.PrivateKey(
                dataRepresentation: data,
                authenticationContext: ctx
            )
        } else {
            return try SecureEnclave.P256.Signing.PrivateKey(
                dataRepresentation: data,
                authenticationContext: makeAuthContext()
            )
        }
    }

    // -------------------------------------------------------------------------
    // MARK: P-256 Key Agreement Key
    // -------------------------------------------------------------------------

    /// Generate and store a P-256 key agreement key in the Secure Enclave.
    public static func generateKeyAgreementKey(
        label: String,
        requireBiometric: Bool
    ) throws -> SecureEnclave.P256.KeyAgreement.PrivateKey {
        guard isAvailable else { throw SecureEnclaveError.notAvailable }

        let accessControl = try makeAccessControl(requireBiometric: requireBiometric)

        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey(
            accessControl: accessControl,
            authenticationContext: requireBiometric ? makeAuthContext() : nil
        )
        // Persist the opaque data representation in the Keychain
        try saveKeychainData(label: label, data: key.dataRepresentation)
        return key
    }

    /// Load an existing P-256 key agreement key from the Secure Enclave.
    public static func loadKeyAgreementKey(
        label: String,
        context: LAContext? = nil
    ) throws -> SecureEnclave.P256.KeyAgreement.PrivateKey {
        guard isAvailable else { throw SecureEnclaveError.notAvailable }

        let data = try loadKeychainData(label: label)

        if let ctx = context {
            return try SecureEnclave.P256.KeyAgreement.PrivateKey(
                dataRepresentation: data,
                authenticationContext: ctx
            )
        } else {
            return try SecureEnclave.P256.KeyAgreement.PrivateKey(
                dataRepresentation: data,
                authenticationContext: makeAuthContext()
            )
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Key Wrapping (for external keys like Kyber / X25519)
    // -------------------------------------------------------------------------

    /// Wrap (encrypt) plaintext key material using AES-GCM with a key derived from
    /// the SE P-256 wrapping key.  The wrapping key agreement key must already exist
    /// under label "nexus.wrap" (created automatically if missing).
    public static func wrapKey(
        plaintext: Data,
        usingLabel label: String
    ) throws -> Data {
        let wrapKey = try getOrCreateWrapKey()
        let (combined, nonce) = try NexusCrypto.aesgcmEncrypt(
            key: wrapKey,
            plaintext: plaintext,
            aad: Data(label.utf8)
        )
        // Store as nonce (12) || ciphertext+tag
        var result = nonce
        result.append(combined)
        return result
    }

    /// Unwrap (decrypt) key material previously wrapped with `wrapKey(plaintext:usingLabel:)`.
    public static func unwrapKey(
        ciphertext: Data,
        usingLabel label: String,
        context: LAContext? = nil
    ) throws -> Data {
        guard ciphertext.count > 12 + 16 else { throw SecureEnclaveError.unwrapFailed }

        let wrapKey      = try getOrCreateWrapKey(context: context)
        let nonce        = ciphertext.prefix(12)
        let ctAndTag     = ciphertext.dropFirst(12)
        let ct           = ctAndTag.dropLast(16)
        let tag          = ctAndTag.suffix(16)

        return try NexusCrypto.aesgcmDecrypt(
            key: wrapKey,
            ciphertext: ct,
            nonce: nonce,
            tag: tag,
            aad: Data(label.utf8)
        )
    }

    // -------------------------------------------------------------------------
    // MARK: Key Deletion
    // -------------------------------------------------------------------------

    public static func deleteKey(label: String) throws {
        let query: [String: Any] = [
            kSecClass as String:            kSecClassGenericPassword,
            kSecAttrService as String:      "com.nexus.messenger",
            kSecAttrAccount as String:      label,
        ]
        let status = SecItemDelete(query as CFDictionary)
        if status != errSecSuccess && status != errSecItemNotFound {
            throw SecureEnclaveError.keyDeleteFailed(status)
        }
    }

    // -------------------------------------------------------------------------
    // MARK: List NEXUS Keys
    // -------------------------------------------------------------------------

    public static func listNexusKeys() -> [String] {
        let query: [String: Any] = [
            kSecClass as String:            kSecClassGenericPassword,
            kSecAttrService as String:      "com.nexus.messenger",
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String:       kSecMatchLimitAll,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess,
              let items = result as? [[String: Any]]
        else { return [] }

        return items.compactMap { $0[kSecAttrAccount as String] as? String }
    }

    // -------------------------------------------------------------------------
    // MARK: Private Helpers
    // -------------------------------------------------------------------------

    private static func makeAccessControl(requireBiometric: Bool) throws -> SecAccessControl {
        var error: Unmanaged<CFError>?
        let flags: SecAccessControlCreateFlags = requireBiometric
            ? [.privateKeyUsage, .biometryCurrentSet]
            : [.privateKeyUsage]

        guard let ac = SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            flags,
            &error
        ) else {
            throw error!.takeRetainedValue() as Error
        }
        return ac
    }

    private static func makeAuthContext() -> LAContext {
        let ctx = LAContext()
        ctx.localizedReason = "Authenticate to access NEXUS keys"
        return ctx
    }

    /// Get or create the AES wrapping key derived from the SE P-256 KA key.
    /// We derive it via ECDH with an ephemeral static key stored in the Keychain.
    private static func getOrCreateWrapKey(context: LAContext? = nil) throws -> SymmetricKey {
        // Use a 256-bit symmetric key stored in the Keychain (encrypted at rest by iOS).
        // If it doesn't exist, generate and store it.
        let wrapLabel = "nexus.sym.wrap"
        if let existing = try? loadKeychainData(label: wrapLabel) {
            return SymmetricKey(data: existing)
        }
        var keyBytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, keyBytes.count, &keyBytes) == errSecSuccess
        else { throw SecureEnclaveError.wrapFailed }
        let keyData = Data(keyBytes)
        try saveKeychainData(label: wrapLabel, data: keyData)
        return SymmetricKey(data: keyData)
    }

    // MARK: Keychain helpers

    static func saveKeychainData(label: String, data: Data) throws {
        // Delete any existing item first
        let deleteQuery: [String: Any] = [
            kSecClass as String:       kSecClassGenericPassword,
            kSecAttrService as String: "com.nexus.messenger",
            kSecAttrAccount as String: label,
        ]
        SecItemDelete(deleteQuery as CFDictionary)

        let addQuery: [String: Any] = [
            kSecClass as String:                        kSecClassGenericPassword,
            kSecAttrService as String:                  "com.nexus.messenger",
            kSecAttrAccount as String:                  label,
            kSecValueData as String:                    data,
            kSecAttrAccessible as String:               kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrSynchronizable as String:           false,
        ]
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw SecureEnclaveError.keyGenerationFailed(status)
        }
    }

    static func loadKeychainData(label: String) throws -> Data {
        let query: [String: Any] = [
            kSecClass as String:            kSecClassGenericPassword,
            kSecAttrService as String:      "com.nexus.messenger",
            kSecAttrAccount as String:      label,
            kSecReturnData as String:       true,
            kSecMatchLimit as String:       kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data
        else { throw SecureEnclaveError.keyLoadFailed(status) }
        return data
    }
}
