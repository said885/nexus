import Foundation
import Network
import CryptoKit

// MARK: - Types

public enum ConnectionStatus: Equatable {
    case disconnected
    case connecting
    case connected
    case reconnecting(attempt: Int)
}

public enum RelayClientError: Error, LocalizedError {
    case notConnected
    case encodingFailed
    case decodingFailed
    case challengeFailed
    case serverError(String)
    case maxReconnectsReached

    public var errorDescription: String? {
        switch self {
        case .notConnected:          return "Not connected to relay"
        case .encodingFailed:        return "Message encoding failed"
        case .decodingFailed:        return "Message decoding failed"
        case .challengeFailed:       return "Server challenge failed"
        case .serverError(let m):    return "Server error: \(m)"
        case .maxReconnectsReached:  return "Maximum reconnect attempts reached"
        }
    }
}

// MARK: - Wire protocol types

private struct RelayEnvelope: Codable {
    let type: String
    let payload: [String: String]
}

// MARK: - RelayClient

public final class RelayClient: NSObject, ObservableObject {

    // -------------------------------------------------------------------------
    // MARK: Published state
    // -------------------------------------------------------------------------

    @Published public private(set) var isConnected = false
    @Published public private(set) var connectionStatus: ConnectionStatus = .disconnected

    // -------------------------------------------------------------------------
    // MARK: Callbacks
    // -------------------------------------------------------------------------

    public var onMessageReceived: ((Data) -> Void)?
    public var onConnected: (() -> Void)?
    public var onDisconnected: (() -> Void)?

    // -------------------------------------------------------------------------
    // MARK: Private state
    // -------------------------------------------------------------------------

    private var webSocketTask: URLSessionWebSocketTask?
    private lazy var urlSession: URLSession = {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest  = 30
        config.timeoutIntervalForResource = 300
        config.waitsForConnectivity       = true
        return URLSession(configuration: config, delegate: self, delegateQueue: .main)
    }()

    private let serverURL: URL
    private var identityHash: String = ""
    private var reconnectAttempts = 0
    private let maxReconnectAttempts = 10
    private var reconnectTask: Task<Void, Never>?
    private var pingTask:      Task<Void, Never>?
    private var receiveTask:   Task<Void, Never>?

    // -------------------------------------------------------------------------
    // MARK: Init
    // -------------------------------------------------------------------------

    public init(serverURL: URL) {
        self.serverURL = serverURL
        super.init()
    }

    // -------------------------------------------------------------------------
    // MARK: Connect / Disconnect
    // -------------------------------------------------------------------------

    public func connect(identityHash: String) async throws {
        self.identityHash = identityHash

        await MainActor.run {
            connectionStatus = .connecting
        }

        var request = URLRequest(url: serverURL)
        request.setValue("nexus-v1", forHTTPHeaderField: "Sec-WebSocket-Protocol")
        request.setValue(identityHash, forHTTPHeaderField: "X-Nexus-Identity")

        let task = urlSession.webSocketTask(with: request)
        webSocketTask = task
        task.resume()

        // Perform server challenge before marking connected
        try await performChallenge(nonce: UUID().uuidString, identityHash: identityHash)

        reconnectAttempts = 0

        await MainActor.run {
            isConnected      = true
            connectionStatus = .connected
        }

        onConnected?()
        startReceiving()
        startPingTimer()
    }

    public func disconnect() {
        reconnectTask?.cancel()
        pingTask?.cancel()
        receiveTask?.cancel()

        webSocketTask?.cancel(with: .normalClosure, reason: nil)
        webSocketTask = nil

        DispatchQueue.main.async { [weak self] in
            self?.isConnected      = false
            self?.connectionStatus = .disconnected
        }
        onDisconnected?()
    }

    // -------------------------------------------------------------------------
    // MARK: Send
    // -------------------------------------------------------------------------

    /// Send an already-encrypted envelope to a recipient.
    public func sendMessage(recipientHash: String, sealedContent: Data) async throws {
        guard let task = webSocketTask, isConnected else {
            throw RelayClientError.notConnected
        }

        let envelope = RelayEnvelope(
            type: "message",
            payload: [
                "to":      recipientHash,
                "from":    identityHash,
                "content": sealedContent.base64EncodedString(),
                "ts":      "\(Date().timeIntervalSince1970)",
            ]
        )

        let data = try JSONEncoder().encode(envelope)
        try await task.send(.data(data))
    }

    // -------------------------------------------------------------------------
    // MARK: Private – receive loop
    // -------------------------------------------------------------------------

    private func startReceiving() {
        receiveTask?.cancel()
        receiveTask = Task { [weak self] in
            await self?.receiveLoop()
        }
    }

    private func receiveLoop() async {
        guard let task = webSocketTask else { return }

        while !Task.isCancelled, isConnected {
            do {
                let message = try await task.receive()
                switch message {
                case .data(let data):
                    handleIncomingData(data)
                case .string(let string):
                    guard let data = string.data(using: .utf8) else { continue }
                    handleIncomingData(data)
                @unknown default:
                    break
                }
            } catch {
                if !Task.isCancelled {
                    await handleDisconnect(error: error)
                }
                return
            }
        }
    }

    private func handleIncomingData(_ data: Data) {
        guard let envelope = try? JSONDecoder().decode(RelayEnvelope.self, from: data) else {
            // Raw encrypted payload — pass directly to callback
            onMessageReceived?(data)
            return
        }

        switch envelope.type {
        case "message":
            if let contentB64 = envelope.payload["content"],
               let content = Data(base64Encoded: contentB64) {
                onMessageReceived?(content)
            }
        case "ping":
            Task { [weak self] in
                try? await self?.sendPong()
            }
        case "error":
            break // handle server errors
        default:
            break
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Private – challenge/response
    // -------------------------------------------------------------------------

    /// NEXUS relay challenge: server sends a random nonce, client signs it with its
    /// P-256 identity key and returns the signature.  This proves ownership of the
    /// claimed identity hash without revealing any message keys.
    private func performChallenge(nonce: String, identityHash: String) async throws {
        // In a full implementation, the server sends a "challenge" message with a nonce
        // and the client replies with a P-256 signature over SHA-256(nonce || identityHash).
        // The server verifies against the stored public key for that identity hash.
        //
        // Here we model the round-trip but omit the actual signing call, which requires
        // access to the SecureEnclave signing key (loaded separately in AppStateManager).
        //
        // Wire format:
        //   client → {"type":"hello","payload":{"identity":"<hash>","nonce":"<uuid>"}}
        //   server → {"type":"challenge","payload":{"serverNonce":"<random>"}}
        //   client → {"type":"auth","payload":{"sig":"<base64 P-256 sig>"}}
        //   server → {"type":"auth_ok","payload":{}}

        guard let task = webSocketTask else { throw RelayClientError.notConnected }

        let hello = RelayEnvelope(
            type: "hello",
            payload: ["identity": identityHash, "nonce": nonce]
        )
        let helloData = try JSONEncoder().encode(hello)
        try await task.send(.data(helloData))

        // Read challenge response (with 10-second timeout)
        let challengeMsg = try await withTimeout(seconds: 10) {
            try await task.receive()
        }

        guard case .data(let challengeData) = challengeMsg,
              let challengeEnv = try? JSONDecoder().decode(RelayEnvelope.self, from: challengeData),
              challengeEnv.type == "challenge" || challengeEnv.type == "auth_ok"
        else {
            // Relay may not implement challenge yet — accept auth_ok directly
            return
        }

        // If the server did send a challenge, respond with a stub signature.
        // TODO: Load signing key from Secure Enclave and produce a real P-256 signature.
        let authEnv = RelayEnvelope(
            type: "auth",
            payload: ["sig": "stub_sig_replace_with_se_p256_signature"]
        )
        let authData = try JSONEncoder().encode(authEnv)
        try await task.send(.data(authData))
    }

    // -------------------------------------------------------------------------
    // MARK: Private – ping/pong
    // -------------------------------------------------------------------------

    private func startPingTimer() {
        pingTask?.cancel()
        pingTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 30 * 1_000_000_000)
                guard let self = self, self.isConnected else { break }
                try? await self.sendPing()
            }
        }
    }

    private func sendPing() async throws {
        let ping = RelayEnvelope(type: "ping", payload: ["ts": "\(Date().timeIntervalSince1970)"])
        let data = try JSONEncoder().encode(ping)
        try await webSocketTask?.send(.data(data))
    }

    private func sendPong() async throws {
        let pong = RelayEnvelope(type: "pong", payload: ["ts": "\(Date().timeIntervalSince1970)"])
        let data = try JSONEncoder().encode(pong)
        try await webSocketTask?.send(.data(data))
    }

    // -------------------------------------------------------------------------
    // MARK: Private – reconnect
    // -------------------------------------------------------------------------

    private func handleDisconnect(error: Error) async {
        await MainActor.run { [weak self] in
            guard let self = self else { return }
            self.isConnected = false
        }
        onDisconnected?()
        scheduleReconnect()
    }

    private func scheduleReconnect() {
        guard reconnectAttempts < maxReconnectAttempts else {
            DispatchQueue.main.async { [weak self] in
                self?.connectionStatus = .disconnected
            }
            return
        }

        reconnectAttempts += 1
        let attempt = reconnectAttempts

        DispatchQueue.main.async { [weak self] in
            self?.connectionStatus = .reconnecting(attempt: attempt)
        }

        // Exponential back-off: 2^attempt seconds, capped at 60s
        let delay = min(pow(2.0, Double(attempt)), 60.0)

        reconnectTask?.cancel()
        reconnectTask = Task { [weak self] in
            guard let self = self else { return }
            try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
            guard !Task.isCancelled else { return }
            try? await self.connect(identityHash: self.identityHash)
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Utility
    // -------------------------------------------------------------------------

    private func withTimeout<T>(seconds: Double, operation: @escaping () async throws -> T) async throws -> T {
        try await withThrowingTaskGroup(of: T.self) { group in
            group.addTask { try await operation() }
            group.addTask {
                try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
                throw RelayClientError.challengeFailed
            }
            let result = try await group.next()!
            group.cancelAll()
            return result
        }
    }
}

// MARK: - URLSessionWebSocketDelegate

extension RelayClient: URLSessionWebSocketDelegate {

    public func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocketTask,
        didOpenWithProtocol protocol: String?
    ) {
        // Connection opened — challenge flow continues in connect()
    }

    public func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocketTask,
        didCloseWith closeCode: URLSessionWebSocketTask.CloseCode,
        reason: Data?
    ) {
        Task { [weak self] in
            await self?.handleDisconnect(error: RelayClientError.notConnected)
        }
    }

    public func urlSession(
        _ session: URLSession,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        // Certificate pinning: in production, validate the server's certificate
        // against a pinned public key or certificate hash stored in the app bundle.
        // For now, use the default handling.
        if challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
           let serverTrust = challenge.protectionSpace.serverTrust {
            // TODO: implement certificate / public key pinning here
            let credential = URLCredential(trust: serverTrust)
            completionHandler(.useCredential, credential)
        } else {
            completionHandler(.performDefaultHandling, nil)
        }
    }
}
