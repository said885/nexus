import CryptoKit
import Foundation
import Security

// MARK: - Identity

/// Represents a user's full cryptographic identity.
/// Kyber keys are stored as raw Data; the private key bytes are wrapped (encrypted) by the
/// Secure Enclave before being persisted — see SecureEnclaveManager.wrapKey/unwrapKey.
public struct NexusIdentity: Codable {
    /// Kyber1024 public key (stub: 32 random bytes — see TODO below)
    public let kyberPublicKey: Data
    /// Kyber1024 private key, wrapped/encrypted by Secure Enclave key "nexus.kyber"
    public let kyberPrivateKeyWrapped: Data
    /// X25519 (Curve25519) public key
    public let x25519PublicKey: Data
    /// P-256 signing public key (DER/raw representation from CryptoKit)
    public let signingPublicKey: Data
    /// SHA-256 of (kyberPublicKey || x25519PublicKey || signingPublicKey)
    public let identityHash: Data
    /// Human-readable fingerprint — 16 hex pairs separated by spaces
    public var fingerprint: String {
        identityHash.prefix(16)
            .map { String(format: "%02X", $0) }
            .chunked(into: 2)
            .map { $0.joined() }
            .joined(separator: " ")
    }
}

private extension Array {
    func chunked(into size: Int) -> [[Element]] {
        stride(from: 0, to: count, by: size).map {
            Array(self[$0 ..< Swift.min($0 + size, count)])
        }
    }
}

// MARK: - Ratchet State

public struct SkippedKeyID: Hashable {
    public let dhPub: Data
    public let msgN: UInt32
}

public struct RatchetState {
    public var rootKey: SymmetricKey
    public var sendChainKey: SymmetricKey?
    public var recvChainKey: SymmetricKey?
    public var sendMsgN: UInt32 = 0
    public var recvMsgN: UInt32 = 0
    public var prevSendCount: UInt32 = 0
    public var dhSendPriv: Curve25519.KeyAgreement.PrivateKey
    public var dhRemotePub: Curve25519.KeyAgreement.PublicKey?
    /// Skipped message keys (limited to 2000 entries to bound memory)
    public var skippedKeys: [SkippedKeyID: SymmetricKey] = [:]

    public static let maxSkip: Int = 2000
}

// MARK: - Encrypted Message

public struct EncryptedMessage: Codable {
    /// Sender's current ratchet public key (X25519)
    public let dhPublic: Data
    public let msgN: UInt32
    public let prevChainLen: UInt32
    public let ciphertext: Data
    public let nonce: Data
    public let tag: Data
}

// MARK: - Errors

public enum NexusCryptoError: Error, LocalizedError {
    case keyGenerationFailed
    case dhFailed
    case encryptionFailed
    case decryptionFailed
    case authTagMismatch
    case skipLimitExceeded
    case missingChainKey
    case missingRemotePublicKey
    case invalidKeySize
    case keyNotFound

    public var errorDescription: String? {
        switch self {
        case .keyGenerationFailed:      return "Key generation failed"
        case .dhFailed:                 return "Diffie-Hellman operation failed"
        case .encryptionFailed:         return "Encryption failed"
        case .decryptionFailed:         return "Decryption failed"
        case .authTagMismatch:          return "Authentication tag mismatch — message tampered"
        case .skipLimitExceeded:        return "Too many skipped messages"
        case .missingChainKey:          return "Chain key not initialised"
        case .missingRemotePublicKey:   return "Remote public key not set"
        case .invalidKeySize:           return "Invalid key size"
        case .keyNotFound:              return "Key not found"
        }
    }
}

// MARK: - NexusCrypto

public final class NexusCrypto {

    private init() {}

    // -------------------------------------------------------------------------
    // MARK: Identity Generation
    // -------------------------------------------------------------------------

    /// Generate a full NEXUS identity.
    /// - The X25519 and P-256 keys live in the Secure Enclave (via SecureEnclaveManager).
    /// - The Kyber private key is generated in software and wrapped by the SE key.
    public static func generateIdentity() throws -> NexusIdentity {
        // 1. P-256 signing key (Secure Enclave)
        let signingKey = try SecureEnclaveManager.generateSigningKey(
            label: "nexus.signing",
            requireBiometric: true
        )
        let signingPub = signingKey.publicKey.derRepresentation

        // 2. X25519 key agreement key
        //    Secure Enclave only supports P-256 for key agreement; we generate
        //    a Curve25519 software key and wrap the private bytes in the SE.
        let x25519Priv = Curve25519.KeyAgreement.PrivateKey()
        let x25519Pub  = x25519Priv.publicKey.rawRepresentation

        // 3. Kyber1024 key pair (stub — replace with liboqs)
        let (kyberPub, kyberPriv) = try generateKyberKeyPair()

        // 4. Wrap Curve25519 + Kyber private keys with SE
        let wrappedX25519 = try SecureEnclaveManager.wrapKey(
            plaintext: x25519Priv.rawRepresentation,
            usingLabel: "nexus.x25519"
        )
        _ = wrappedX25519 // stored alongside identity; omitted from struct for brevity —
                          // in production, NexusIdentity should carry wrappedX25519PrivateKey too.

        let wrappedKyber = try SecureEnclaveManager.wrapKey(
            plaintext: kyberPriv,
            usingLabel: "nexus.kyber"
        )

        // 5. Identity hash
        var combined = Data()
        combined.append(kyberPub)
        combined.append(x25519Pub)
        combined.append(signingPub)
        let hash = identityHash(combined)

        return NexusIdentity(
            kyberPublicKey: kyberPub,
            kyberPrivateKeyWrapped: wrappedKyber,
            x25519PublicKey: x25519Pub,
            signingPublicKey: signingPub,
            identityHash: hash
        )
    }

    // -------------------------------------------------------------------------
    // MARK: Kyber1024 (STUB)
    // -------------------------------------------------------------------------

    // TODO: Replace with production Kyber1024 implementation.
    // Recommended path: integrate liboqs via Swift Package Manager
    //   .package(url: "https://github.com/open-quantum-safe/liboqs-swift", from: "0.8.0")
    // then call OQS_KEM_kyber_1024_keypair / OQS_KEM_kyber_1024_enc / OQS_KEM_kyber_1024_dec.
    //
    // The stubs below produce RANDOM BYTES and provide NO post-quantum security.
    // They exist solely to allow the rest of the application to compile and run.

    /// Generate a Kyber1024 key pair.
    /// STUB: returns 1568 bytes of random data as public key, 3168 as private key.
    public static func generateKyberKeyPair() throws -> (publicKey: Data, privateKey: Data) {
        // TODO: Replace with production Kyber1024 implementation
        var pubBytes  = [UInt8](repeating: 0, count: 1568)
        var privBytes = [UInt8](repeating: 0, count: 3168)
        guard SecRandomCopyBytes(kSecRandomDefault, pubBytes.count,  &pubBytes)  == errSecSuccess,
              SecRandomCopyBytes(kSecRandomDefault, privBytes.count, &privBytes) == errSecSuccess
        else { throw NexusCryptoError.keyGenerationFailed }
        return (Data(pubBytes), Data(privBytes))
    }

    /// Kyber1024 encapsulation — derive 32-byte shared secret and ciphertext.
    /// STUB: returns random shared secret and 1568-byte random ciphertext.
    public static func kyberEncapsulate(publicKey: Data) throws -> (sharedSecret: Data, ciphertext: Data) {
        // TODO: Replace with production Kyber1024 implementation
        var sharedSecret = [UInt8](repeating: 0, count: 32)
        var ciphertext   = [UInt8](repeating: 0, count: 1568)
        guard SecRandomCopyBytes(kSecRandomDefault, sharedSecret.count, &sharedSecret) == errSecSuccess,
              SecRandomCopyBytes(kSecRandomDefault, ciphertext.count,   &ciphertext)   == errSecSuccess
        else { throw NexusCryptoError.encryptionFailed }
        return (Data(sharedSecret), Data(ciphertext))
    }

    /// Kyber1024 decapsulation — recover the shared secret from the ciphertext.
    /// STUB: returns random shared secret (will NOT match encapsulation output).
    public static func kyberDecapsulate(privateKey: Data, ciphertext: Data) throws -> Data {
        // TODO: Replace with production Kyber1024 implementation
        var sharedSecret = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, sharedSecret.count, &sharedSecret) == errSecSuccess
        else { throw NexusCryptoError.decryptionFailed }
        return Data(sharedSecret)
    }

    // -------------------------------------------------------------------------
    // MARK: X25519 DH
    // -------------------------------------------------------------------------

    /// Perform X25519 DH and derive a 32-byte SymmetricKey via HKDF-SHA512.
    public static func performDH(
        privateKey: Curve25519.KeyAgreement.PrivateKey,
        publicKey:  Curve25519.KeyAgreement.PublicKey
    ) throws -> SymmetricKey {
        let sharedSecret = try privateKey.sharedSecretFromKeyAgreement(with: publicKey)
        return sharedSecret.hkdfDerivedSymmetricKey(
            using: SHA512.self,
            salt: Data("NexusDH".utf8),
            sharedInfo: Data("NexusDH-v1".utf8),
            outputByteCount: 32
        )
    }

    // -------------------------------------------------------------------------
    // MARK: HKDF
    // -------------------------------------------------------------------------

    /// HKDF-SHA512.
    public static func hkdf(
        inputKey: SymmetricKey,
        salt: Data?,
        info: Data,
        outputLength: Int
    ) -> SymmetricKey {
        let saltKey: SymmetricKey
        if let salt = salt, !salt.isEmpty {
            saltKey = SymmetricKey(data: salt)
        } else {
            // RFC 5869: if no salt, use HashLen zeros
            saltKey = SymmetricKey(data: Data(repeating: 0, count: SHA512.byteCount))
        }
        return HKDF<SHA512>.deriveKey(
            inputKeyMaterial: inputKey,
            salt: saltKey,
            info: info,
            outputByteCount: outputLength
        )
    }

    // -------------------------------------------------------------------------
    // MARK: Double Ratchet - KDF helpers
    // -------------------------------------------------------------------------

    /// KDF_RK: derive new root key and chain key from root key + DH output.
    private static func kdfRK(rootKey: SymmetricKey, dhOut: SymmetricKey) -> (SymmetricKey, SymmetricKey) {
        let derived = hkdf(
            inputKey: dhOut,
            salt: rootKey.withUnsafeBytes { Data($0) },
            info: Data("NexusRatchet-RK".utf8),
            outputLength: 64
        )
        return derived.withUnsafeBytes { ptr -> (SymmetricKey, SymmetricKey) in
            let bytes = Array(ptr)
            let newRK = SymmetricKey(data: Data(bytes[0..<32]))
            let newCK = SymmetricKey(data: Data(bytes[32..<64]))
            return (newRK, newCK)
        }
    }

    /// KDF_CK: derive next chain key and message key from current chain key.
    private static func kdfCK(chainKey: SymmetricKey) -> (nextChainKey: SymmetricKey, msgKey: SymmetricKey) {
        let ckBytes = chainKey.withUnsafeBytes { Data($0) }
        // message key: HMAC(ck, 0x01)
        let msgKey = HKDF<SHA512>.deriveKey(
            inputKeyMaterial: chainKey,
            salt: SymmetricKey(data: ckBytes),
            info: Data([0x01]),
            outputByteCount: 32
        )
        // next chain key: HKDF(ck, ck, 0x02)
        let nextCK = HKDF<SHA512>.deriveKey(
            inputKeyMaterial: chainKey,
            salt: SymmetricKey(data: ckBytes),
            info: Data([0x02]),
            outputByteCount: 32
        )
        return (nextCK, msgKey)
    }

    // -------------------------------------------------------------------------
    // MARK: Double Ratchet - Init
    // -------------------------------------------------------------------------

    /// Initialise the ratchet for the sender (Alice).
    /// `sharedSecret` comes from the initial X3DH (or hybrid PQC+X3DH) handshake.
    public static func initRatchetSender(
        sharedSecret: SymmetricKey,
        remoteRatchetPub: Curve25519.KeyAgreement.PublicKey
    ) throws -> RatchetState {
        let dhSend = Curve25519.KeyAgreement.PrivateKey()
        let dhOut  = try performDH(privateKey: dhSend, publicKey: remoteRatchetPub)
        let (newRK, sendCK) = kdfRK(rootKey: sharedSecret, dhOut: dhOut)

        return RatchetState(
            rootKey: newRK,
            sendChainKey: sendCK,
            recvChainKey: nil,
            sendMsgN: 0,
            recvMsgN: 0,
            prevSendCount: 0,
            dhSendPriv: dhSend,
            dhRemotePub: remoteRatchetPub,
            skippedKeys: [:]
        )
    }

    /// Initialise the ratchet for the receiver (Bob).
    public static func initRatchetReceiver(
        sharedSecret: SymmetricKey,
        localRatchetKey: Curve25519.KeyAgreement.PrivateKey
    ) throws -> RatchetState {
        return RatchetState(
            rootKey: sharedSecret,
            sendChainKey: nil,
            recvChainKey: nil,
            sendMsgN: 0,
            recvMsgN: 0,
            prevSendCount: 0,
            dhSendPriv: localRatchetKey,
            dhRemotePub: nil,
            skippedKeys: [:]
        )
    }

    // -------------------------------------------------------------------------
    // MARK: Double Ratchet - Encrypt
    // -------------------------------------------------------------------------

    public static func ratchetEncrypt(
        state: inout RatchetState,
        plaintext: Data,
        associatedData: Data
    ) throws -> EncryptedMessage {
        guard var ck = state.sendChainKey else { throw NexusCryptoError.missingChainKey }

        let (nextCK, msgKey) = kdfCK(chainKey: ck)
        ck = nextCK
        state.sendChainKey = ck

        let header = buildHeader(
            dhPub: state.dhSendPriv.publicKey.rawRepresentation,
            msgN: state.sendMsgN,
            prevChainLen: state.prevSendCount
        )
        let aad = associatedData + header

        let (ciphertext, nonce) = try aesgcmEncrypt(key: msgKey, plaintext: plaintext, aad: aad)
        // AES-GCM in CryptoKit produces ciphertext+tag together; split for struct
        let ctData  = ciphertext.prefix(ciphertext.count - 16)
        let tagData = ciphertext.suffix(16)

        state.sendMsgN += 1

        return EncryptedMessage(
            dhPublic: state.dhSendPriv.publicKey.rawRepresentation,
            msgN: state.sendMsgN - 1,
            prevChainLen: state.prevSendCount,
            ciphertext: ctData,
            nonce: nonce,
            tag: tagData
        )
    }

    // -------------------------------------------------------------------------
    // MARK: Double Ratchet - Decrypt
    // -------------------------------------------------------------------------

    public static func ratchetDecrypt(
        state: inout RatchetState,
        message: EncryptedMessage,
        associatedData: Data
    ) throws -> Data {
        // Check skipped keys first
        let skipID = SkippedKeyID(dhPub: message.dhPublic, msgN: message.msgN)
        if let skippedKey = state.skippedKeys[skipID] {
            state.skippedKeys.removeValue(forKey: skipID)
            return try decryptWithKey(skippedKey, message: message, associatedData: associatedData)
        }

        let remotePubRaw = message.dhPublic
        let remotePub = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: remotePubRaw)

        // If this is a new DH ratchet step
        let isNewRatchetStep: Bool
        if let currentRemotePub = state.dhRemotePub {
            isNewRatchetStep = currentRemotePub.rawRepresentation != remotePubRaw
        } else {
            isNewRatchetStep = true
        }

        if isNewRatchetStep {
            // Skip messages in the current receive chain
            if let recvCK = state.recvChainKey {
                try skipMessageKeys(state: &state, until: message.prevChainLen, chainKey: recvCK, dhPub: state.dhRemotePub?.rawRepresentation ?? Data())
            }

            // DH ratchet step
            let dhOut1 = try performDH(privateKey: state.dhSendPriv, publicKey: remotePub)
            let (newRK1, recvCK) = kdfRK(rootKey: state.rootKey, dhOut: dhOut1)

            let newDHSend = Curve25519.KeyAgreement.PrivateKey()
            let dhOut2    = try performDH(privateKey: newDHSend, publicKey: remotePub)
            let (newRK2, sendCK) = kdfRK(rootKey: newRK1, dhOut: dhOut2)

            state.prevSendCount = state.sendMsgN
            state.sendMsgN      = 0
            state.recvMsgN      = 0
            state.dhRemotePub   = remotePub
            state.dhSendPriv    = newDHSend
            state.sendChainKey  = sendCK
            state.recvChainKey  = recvCK
            state.rootKey       = newRK2
        }

        guard var recvCK = state.recvChainKey else { throw NexusCryptoError.missingChainKey }

        // Skip to the right message
        try skipMessageKeys(state: &state, until: message.msgN, chainKey: recvCK, dhPub: remotePubRaw)
        if let updatedRecvCK = state.recvChainKey {
            recvCK = updatedRecvCK
        }

        let (nextCK, msgKey) = kdfCK(chainKey: recvCK)
        state.recvChainKey = nextCK
        state.recvMsgN    += 1

        return try decryptWithKey(msgKey, message: message, associatedData: associatedData)
    }

    // -------------------------------------------------------------------------
    // MARK: Private ratchet helpers
    // -------------------------------------------------------------------------

    private static func skipMessageKeys(
        state: inout RatchetState,
        until targetN: UInt32,
        chainKey: SymmetricKey,
        dhPub: Data
    ) throws {
        guard targetN > state.recvMsgN else { return }
        if Int(targetN) - Int(state.recvMsgN) > RatchetState.maxSkip {
            throw NexusCryptoError.skipLimitExceeded
        }
        var ck = chainKey
        while state.recvMsgN < targetN {
            let (nextCK, msgKey) = kdfCK(chainKey: ck)
            let skipID = SkippedKeyID(dhPub: dhPub, msgN: state.recvMsgN)
            state.skippedKeys[skipID] = msgKey
            ck = nextCK
            state.recvMsgN += 1
        }
        state.recvChainKey = ck
    }

    private static func buildHeader(dhPub: Data, msgN: UInt32, prevChainLen: UInt32) -> Data {
        var h = Data()
        h.append(dhPub)
        withUnsafeBytes(of: msgN.bigEndian)      { h.append(contentsOf: $0) }
        withUnsafeBytes(of: prevChainLen.bigEndian) { h.append(contentsOf: $0) }
        return h
    }

    private static func decryptWithKey(
        _ key: SymmetricKey,
        message: EncryptedMessage,
        associatedData: Data
    ) throws -> Data {
        let header = buildHeader(dhPub: message.dhPublic, msgN: message.msgN, prevChainLen: message.prevChainLen)
        let aad    = associatedData + header
        return try aesgcmDecrypt(
            key: key,
            ciphertext: message.ciphertext,
            nonce: message.nonce,
            tag: message.tag,
            aad: aad
        )
    }

    // -------------------------------------------------------------------------
    // MARK: AES-GCM
    // -------------------------------------------------------------------------

    /// Encrypt and return (ciphertext+tag, nonce).
    public static func aesgcmEncrypt(
        key: SymmetricKey,
        plaintext: Data,
        aad: Data
    ) throws -> (Data, Data) {
        do {
            let nonce     = AES.GCM.Nonce()
            let sealedBox = try AES.GCM.seal(plaintext, using: key, nonce: nonce, authenticating: aad)
            let nonceData = Data(nonce)
            // sealedBox.ciphertext + sealedBox.tag
            let combined  = sealedBox.ciphertext + sealedBox.tag
            return (combined, nonceData)
        } catch {
            throw NexusCryptoError.encryptionFailed
        }
    }

    /// Decrypt.  `ciphertext` and `tag` are separate (as stored in EncryptedMessage).
    public static func aesgcmDecrypt(
        key: SymmetricKey,
        ciphertext: Data,
        nonce: Data,
        tag: Data,
        aad: Data
    ) throws -> Data {
        do {
            let gcmNonce  = try AES.GCM.Nonce(data: nonce)
            let sealedBox = try AES.GCM.SealedBox(nonce: gcmNonce, ciphertext: ciphertext, tag: tag)
            return try AES.GCM.open(sealedBox, using: key, authenticating: aad)
        } catch {
            throw NexusCryptoError.decryptionFailed
        }
    }

    // -------------------------------------------------------------------------
    // MARK: X3DH Key Exchange
    // -------------------------------------------------------------------------

    /// Pre-key bundle for X3DH
    public struct PreKeyBundle {
        public let identityKey: Data        // Long-term identity public key (Kyber or X25519)
        public let signedPreKey: Data       // Signed pre-key public
        public let signedPreKeySignature: Data
        public let oneTimePreKey: Data?     // Optional one-time pre-key

        public init(identityKey: Data, signedPreKey: Data, signedPreKeySignature: Data, oneTimePreKey: Data?) {
            self.identityKey = identityKey
            self.signedPreKey = signedPreKey
            self.signedPreKeySignature = signedPreKeySignature
            self.oneTimePreKey = oneTimePreKey
        }
    }

    /// X3DH initiator (Alice) - performs key agreement with Bob's pre-key bundle
    /// Returns (masterSecret, ephemeralPublicKey)
    public static func x3dhInitiate(
        myIdentity: Curve25519.KeyAgreement.PrivateKey,
        myEphemeral: Curve25519.KeyAgreement.PrivateKey,
        theirBundle: PreKeyBundle
    ) throws -> (masterSecret: SymmetricKey, ephemeralPubKey: Data) {
        // 1. DH1 = DH(IKa, SPKb)
        let theirSignedPreKey = try Curve25519.KeyAgreement.PublicKey(
            rawRepresentation: theirBundle.signedPreKey
        )
        let dh1 = try performDH(privateKey: myIdentity, publicKey: theirSignedPreKey)

        // 2. DH2 = DH(EKa, IKb)
        let theirIdentityKey = try Curve25519.KeyAgreement.PublicKey(
            rawRepresentation: theirBundle.identityKey
        )
        let dh2 = try performDH(privateKey: myEphemeral, publicKey: theirIdentityKey)

        // 3. DH3 = DH(EKa, SPKb)
        let dh3 = try performDH(privateKey: myEphemeral, publicKey: theirSignedPreKey)

        // 4. DH4 = DH(EKa, OPKb) if one-time pre-key exists
        var dh4: SymmetricKey? = nil
        if let opk = theirBundle.oneTimePreKey {
            let theirOPK = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: opk)
            dh4 = try performDH(privateKey: myEphemeral, publicKey: theirOPK)
        }

        // 5. Combine DH outputs
        var combinedSecret = Data()
        combinedSecret.append(contentsOf: dh1.withUnsafeBytes { Data($0) })
        combinedSecret.append(contentsOf: dh2.withUnsafeBytes { Data($0) })
        combinedSecret.append(contentsOf: dh3.withUnsafeBytes { Data($0) })
        if let dh4 = dh4 {
            combinedSecret.append(contentsOf: dh4.withUnsafeBytes { Data($0) })
        }

        // 6. Derive master secret via HKDF
        let masterKey = SymmetricKey(data: combinedSecret)
        let masterSecret = hkdf(
            inputKey: masterKey,
            salt: Data(repeating: 0, count: 32),
            info: Data("NexusX3DH-v1".utf8),
            outputLength: 32
        )

        return (masterSecret, myEphemeral.publicKey.rawRepresentation)
    }

    /// X3DH responder (Bob) - derives shared secret from Alice's ephemeral key
    public static func x3dhRespond(
        myIdentity: Curve25519.KeyAgreement.PrivateKey,
        mySignedPreKey: Curve25519.KeyAgreement.PrivateKey,
        myOneTimePreKey: Curve25519.KeyAgreement.PrivateKey?,
        theirIdentityPubKey: Data,
        theirEphemeralPubKey: Data
    ) throws -> SymmetricKey {
        // 1. DH1 = DH(SPKb, IKa)
        let theirIdentity = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: theirIdentityPubKey)
        let dh1 = try performDH(privateKey: mySignedPreKey, publicKey: theirIdentity)

        // 2. DH2 = DH(IKB, EKa)
        let theirEphemeral = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: theirEphemeralPubKey)
        let dh2 = try performDH(privateKey: myIdentity, publicKey: theirEphemeral)

        // 3. DH3 = DH(SPKb, EKa)
        let dh3 = try performDH(privateKey: mySignedPreKey, publicKey: theirEphemeral)

        // 4. DH4 = DH(OPKb, EKa) if one-time pre-key exists
        var dh4: SymmetricKey? = nil
        if let opk = myOneTimePreKey {
            dh4 = try performDH(privateKey: opk, publicKey: theirEphemeral)
        }

        // 5. Combine DH outputs (same order as initiator)
        var combinedSecret = Data()
        combinedSecret.append(contentsOf: dh1.withUnsafeBytes { Data($0) })
        combinedSecret.append(contentsOf: dh2.withUnsafeBytes { Data($0) })
        combinedSecret.append(contentsOf: dh3.withUnsafeBytes { Data($0) })
        if let dh4 = dh4 {
            combinedSecret.append(contentsOf: dh4.withUnsafeBytes { Data($0) })
        }

        // 6. Derive master secret via HKDF
        let masterKey = SymmetricKey(data: combinedSecret)
        return hkdf(
            inputKey: masterKey,
            salt: Data(repeating: 0, count: 32),
            info: Data("NexusX3DH-v1".utf8),
            outputLength: 32
        )
    }

    /// Sign a pre-key with identity key
    public static func signPreKey(
        preKeyPublic: Data,
        signingKey: P256.Signing.PrivateKey
    ) throws -> Data {
        let signature = try signingKey.signature(for: preKeyPublic)
        return signature.derRepresentation
    }

    /// Verify a pre-key signature
    public static func verifyPreKeySignature(
        preKeyPublic: Data,
        signature: Data,
        identityPublicKey: Data
    ) throws -> Bool {
        let signingKey = try P256.Signing.PublicKey(derRepresentation: identityPublicKey)
        let sig = try P256.Signing.ECDSASignature(derRepresentation: signature)
        return signingKey.isValidSignature(sig, for: preKeyPublic)
    }

    // -------------------------------------------------------------------------
    // MARK: Utilities
    // -------------------------------------------------------------------------

    /// SHA-256 fingerprint of a public key blob.
    public static func identityHash(_ publicKeyData: Data) -> Data {
        Data(SHA256.hash(data: publicKeyData))
    }

    /// Overwrite a Data buffer with zeros then remove all bytes.
    public static func secureWipe(_ data: inout Data) {
        data.resetBytes(in: 0..<data.count)
        data = Data()
    }
}
