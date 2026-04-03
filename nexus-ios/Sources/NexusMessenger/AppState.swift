import CryptoKit
import Combine
import Foundation
import LocalAuthentication
import SwiftUI

// MARK: - AppStateManager

@MainActor
public final class AppStateManager: ObservableObject {

    // -------------------------------------------------------------------------
    // MARK: Published state
    // -------------------------------------------------------------------------

    @Published public var isSetupComplete  = false
    @Published public var isUnlocked       = false
    @Published public var identity: NexusIdentity?
    @Published public var threatLevel: ThreatLevel = .none
    @Published public var conversations: [ConversationModel] = []
    @Published public var errorMessage: String?
    @Published public var isLoading = false

    // -------------------------------------------------------------------------
    // MARK: Private state
    // -------------------------------------------------------------------------

    private var relayClient: RelayClient?
    /// In-memory ratchet sessions keyed by participant identity hash
    private var ratchetSessions: [String: RatchetState] = [:]
    private var messageStore: MessageStore?
    private var cancellables = Set<AnyCancellable>()

    // -------------------------------------------------------------------------
    // MARK: Init
    // -------------------------------------------------------------------------

    public init() {}

    // -------------------------------------------------------------------------
    // MARK: Initialize
    // -------------------------------------------------------------------------

    public func initialize() {
        // 1. Jailbreak / integrity check
        let threat = IntegrityChecker.performFullCheck()
        threatLevel = threat
        IntegrityChecker.handleThreat(threat)

        // 2. Register for emergency wipe
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleEmergencyWipe),
            name: .nexusEmergencyWipe,
            object: nil
        )

        // 3. Check whether a NEXUS identity has already been created
        let keys = SecureEnclaveManager.listNexusKeys()
        isSetupComplete = keys.contains("nexus.signing") && keys.contains("nexus.kyber")

        // 4. Initialise message store
        messageStore = MessageStore()

        // 5. Prevent screenshots (set window's isSecureTextEntry equivalent)
        preventScreenshots()
    }

    // -------------------------------------------------------------------------
    // MARK: Screenshot Prevention
    // -------------------------------------------------------------------------

    private func preventScreenshots() {
        // On iOS, the recommended approach is to add a secure text field to the
        // window's layer, which causes the OS to blank the screen in the app switcher
        // and block screen recording.  This must be called on the main thread.
        //
        // Implementation note: In a UIKit app this would be done in the SceneDelegate.
        // In a pure SwiftUI app we use a UIViewRepresentable shim on the root view.
        // The actual mechanism is invoked via UITextField.isSecureTextEntry set to true
        // on an invisible text field added to the key window — see SecureWindowModifier.
    }

    // -------------------------------------------------------------------------
    // MARK: Generate Identity
    // -------------------------------------------------------------------------

    public func generateIdentity() async throws {
        isLoading = true
        defer { isLoading = false }

        do {
            let id = try NexusCrypto.generateIdentity()
            // Persist identity public data
            let encoder = JSONEncoder()
            let idData  = try encoder.encode(id)
            try SecureEnclaveManager.saveKeychainData(label: "nexus.identity.pub", data: idData)
            identity       = id
            isSetupComplete = true
        } catch {
            errorMessage = error.localizedDescription
            throw error
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Biometric Unlock
    // -------------------------------------------------------------------------

    public func unlockWithBiometrics() async throws {
        isLoading = true
        defer { isLoading = false }

        let context = LAContext()
        var authError: NSError?

        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &authError) else {
            // Fall back to passcode
            try await unlockWithPasscode(context: context)
            return
        }

        let reason = "Authenticate to unlock NEXUS"
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: reason) { success, error in
                if success {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? NSError(domain: "LAError", code: -1))
                }
            }
        }

        // Load identity from Keychain
        try loadIdentity()
        isUnlocked = true
    }

    private func unlockWithPasscode(context: LAContext) async throws {
        let reason = "Authenticate to unlock NEXUS"
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason) { success, error in
                if success {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: error ?? NSError(domain: "LAError", code: -1))
                }
            }
        }
        try loadIdentity()
        isUnlocked = true
    }

    private func loadIdentity() throws {
        let data = try SecureEnclaveManager.loadKeychainData(label: "nexus.identity.pub")
        identity  = try JSONDecoder().decode(NexusIdentity.self, from: data)
        // Refresh conversations
        if let store = messageStore {
            conversations = (try? store.fetchConversations()) ?? []
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Send Message
    // -------------------------------------------------------------------------

    public func sendMessage(to recipientHash: String, text: String) async throws {
        guard let relay = relayClient, relay.isConnected else {
            throw RelayClientError.notConnected
        }
        guard let myIdentity = identity else {
            throw NexusCryptoError.keyNotFound
        }

        // Get or initialise ratchet session
        let encrypted: EncryptedMessage
        if var session = ratchetSessions[recipientHash] {
            encrypted = try NexusCrypto.ratchetEncrypt(
                state: &session,
                plaintext: Data(text.utf8),
                associatedData: Data(myIdentity.identityHash)
            )
            ratchetSessions[recipientHash] = session
        } else {
            // No session yet — use a fresh ephemeral shared secret (placeholder for X3DH)
            // TODO: perform full X3DH handshake before first message
            let sharedSecret = SymmetricKey(size: .bits256)
            let remotePub    = Curve25519.KeyAgreement.PrivateKey().publicKey // placeholder
            var session      = try NexusCrypto.initRatchetSender(
                sharedSecret: sharedSecret,
                remoteRatchetPub: remotePub
            )
            encrypted = try NexusCrypto.ratchetEncrypt(
                state: &session,
                plaintext: Data(text.utf8),
                associatedData: Data(myIdentity.identityHash)
            )
            ratchetSessions[recipientHash] = session
        }

        // Serialise and send
        let sealedData = try JSONEncoder().encode(encrypted)
        try await relay.sendMessage(recipientHash: recipientHash, sealedContent: sealedData)

        // Persist
        let conversationId = conversationID(for: recipientHash)
        _ = try messageStore?.saveMessage(
            conversationId: conversationId,
            participantHash: recipientHash,
            content: text,
            isSent: true
        )

        conversations = (try? messageStore?.fetchConversations()) ?? conversations
    }

    // -------------------------------------------------------------------------
    // MARK: Receive Message
    // -------------------------------------------------------------------------

    private func handleIncomingMessage(_ data: Data) {
        guard let myIdentity = identity else { return }

        // Attempt to decode as EncryptedMessage
        guard let encrypted = try? JSONDecoder().decode(EncryptedMessage.self, from: data) else {
            return
        }

        // We need to know the sender to look up the ratchet session.
        // In a full implementation the relay envelope carries the sender's identity hash.
        // Here we use a placeholder.
        let senderHash = "unknown" // TODO: extract from relay envelope

        guard var session = ratchetSessions[senderHash] else { return }

        guard let plaintext = try? NexusCrypto.ratchetDecrypt(
            state: &session,
            message: encrypted,
            associatedData: Data(myIdentity.identityHash)
        ),
        let text = String(data: plaintext, encoding: .utf8)
        else { return }

        ratchetSessions[senderHash] = session

        let conversationId = conversationID(for: senderHash)
        _ = try? messageStore?.saveMessage(
            conversationId: conversationId,
            participantHash: senderHash,
            content: text,
            isSent: false
        )

        DispatchQueue.main.async { [weak self] in
            self?.conversations = (try? self?.messageStore?.fetchConversations()) ?? (self?.conversations ?? [])
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Connect to Relay
    // -------------------------------------------------------------------------

    public func connectToRelay(url: URL) async throws {
        guard let id = identity else { throw NexusCryptoError.keyNotFound }

        let relay = RelayClient(serverURL: url)
        relay.onMessageReceived = { [weak self] data in
            self?.handleIncomingMessage(data)
        }

        try await relay.connect(identityHash: id.identityHash.hexString)
        relayClient = relay

        // Mirror published isConnected
        relay.$isConnected
            .receive(on: DispatchQueue.main)
            .sink { _ in }
            .store(in: &cancellables)
    }

    // -------------------------------------------------------------------------
    // MARK: Panic Wipe
    // -------------------------------------------------------------------------

    public func triggerPanicWipe() {
        IntegrityChecker.performEmergencyWipe()
        isUnlocked       = false
        isSetupComplete  = false
        identity         = nil
        ratchetSessions  = [:]
        conversations    = []
    }

    // -------------------------------------------------------------------------
    // MARK: Private helpers
    // -------------------------------------------------------------------------

    private func conversationID(for participantHash: String) -> String {
        guard let myHash = identity?.identityHash.hexString else { return participantHash }
        // Deterministic conversation ID: sorted concatenation of both hashes
        let hashes = [myHash, participantHash].sorted()
        return Data(SHA256.hash(data: Data((hashes[0] + hashes[1]).utf8))).hexString
    }

    @objc private func handleEmergencyWipe() {
        triggerPanicWipe()
    }
}

// MARK: - Data hex helper

private extension Data {
    var hexString: String {
        map { String(format: "%02x", $0) }.joined()
    }
}
