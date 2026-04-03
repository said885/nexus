package com.nexus.messenger.network

import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

private const val TAG = "RelayClient"
private const val MAX_RECONNECT_DELAY_MS = 30_000L
private const val INITIAL_RECONNECT_DELAY_MS = 1_000L
private const val PING_INTERVAL_SEC = 30L
private const val MAX_QUEUE_SIZE = 500

/**
 * Message envelope sent over the relay WebSocket.
 * Format: [1 byte type][recipient hash 32 bytes][content length 4 bytes][content...]
 */
enum class RelayMessageType(val code: Byte) {
    REGISTER(0x01),
    SEND(0x02),
    RECEIVE(0x03),
    ACK(0x04),
    CHALLENGE(0x05),
    CHALLENGE_RESPONSE(0x06),
    PING(0x07),
    PONG(0x08),
    ERROR(0xFF.toByte()),
}

data class QueuedMessage(
    val recipientHash: ByteArray,
    val sealedContent: ByteArray,
    val attempts: Int = 0,
)

class RelayClient(
    private val pinnedCertSha256: String? = null,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private var webSocket: WebSocket? = null
    private var serverUrl: String? = null
    private var myIdentityHash: String? = null

    private val isConnected = AtomicBoolean(false)
    private val isReconnecting = AtomicBoolean(false)
    private val reconnectDelay = AtomicInteger(INITIAL_RECONNECT_DELAY_MS.toInt())

    private val sendQueue = ConcurrentLinkedQueue<QueuedMessage>()
    private var reconnectJob: Job? = null
    private var drainJob: Job? = null

    // Callbacks
    var onMessageReceived: ((senderHash: ByteArray, content: ByteArray) -> Unit)? = null
    var onConnectionStateChanged: ((connected: Boolean) -> Unit)? = null
    var onError: ((Throwable) -> Unit)? = null

    private val httpClient: OkHttpClient by lazy {
        OkHttpClient.Builder()
            .pingInterval(PING_INTERVAL_SEC, TimeUnit.SECONDS)
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(0, TimeUnit.SECONDS) // No read timeout for WebSocket
            .writeTimeout(30, TimeUnit.SECONDS)
            .apply {
                if (pinnedCertSha256 != null) {
                    // Certificate pinning via CertificatePinner
                    val serverHost = serverUrl?.removePrefix("wss://")
                        ?.removePrefix("ws://")
                        ?.substringBefore("/") ?: ""
                    if (serverHost.isNotEmpty()) {
                        certificatePinner(
                            okhttp3.CertificatePinner.Builder()
                                .add(serverHost, "sha256/$pinnedCertSha256")
                                .build()
                        )
                    }
                }
            }
            .build()
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    fun connect(serverUrl: String, identityHash: String) {
        this.serverUrl = serverUrl
        this.myIdentityHash = identityHash
        doConnect()
    }

    fun sendMessage(recipientHash: ByteArray, sealedContent: ByteArray) {
        if (sendQueue.size >= MAX_QUEUE_SIZE) {
            Log.w(TAG, "Send queue full, dropping oldest message")
            sendQueue.poll()
        }
        sendQueue.offer(QueuedMessage(recipientHash, sealedContent))
        if (isConnected.get()) {
            drainQueue()
        }
    }

    fun disconnect() {
        reconnectJob?.cancel()
        drainJob?.cancel()
        isReconnecting.set(false)
        webSocket?.close(1000, "Client disconnect")
        webSocket = null
        isConnected.set(false)
    }

    fun isConnected(): Boolean = isConnected.get()

    // -------------------------------------------------------------------------
    // Connection logic
    // -------------------------------------------------------------------------

    private fun doConnect() {
        val url = serverUrl ?: return
        val identityHash = myIdentityHash ?: return

        try {
            val request = Request.Builder()
                .url(url)
                .header("X-Nexus-Identity", identityHash)
                .header("X-Nexus-Version", "1")
                .build()

            webSocket = httpClient.newWebSocket(request, createListener())
        } catch (e: Exception) {
            Log.e(TAG, "Connection failed", e)
            scheduleReconnect()
        }
    }

    private fun createListener(): WebSocketListener = object : WebSocketListener() {
        override fun onOpen(webSocket: WebSocket, response: Response) {
            Log.d(TAG, "WebSocket connected")
            isConnected.set(true)
            reconnectDelay.set(INITIAL_RECONNECT_DELAY_MS.toInt())
            isReconnecting.set(false)
            onConnectionStateChanged?.invoke(true)

            // Register identity
            sendRegistration()
            // Drain queued messages
            drainQueue()
        }

        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
            handleIncomingBytes(bytes.toByteArray())
        }

        override fun onMessage(webSocket: WebSocket, text: String) {
            // We use binary only; ignore text frames
            Log.w(TAG, "Unexpected text frame received")
        }

        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
            Log.d(TAG, "WebSocket closing: $code $reason")
            webSocket.close(1000, null)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            Log.d(TAG, "WebSocket closed: $code $reason")
            handleDisconnect()
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            Log.e(TAG, "WebSocket failure", t)
            onError?.invoke(t)
            handleDisconnect()
        }
    }

    private fun handleDisconnect() {
        isConnected.set(false)
        onConnectionStateChanged?.invoke(false)
        scheduleReconnect()
    }

    private fun scheduleReconnect() {
        if (serverUrl == null || isReconnecting.getAndSet(true)) return
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            val delay = reconnectDelay.get().toLong()
            Log.d(TAG, "Reconnecting in ${delay}ms")
            delay(delay)

            // Exponential backoff with jitter
            val nextDelay = minOf(delay * 2, MAX_RECONNECT_DELAY_MS) +
                    (Math.random() * 1000).toLong()
            // CRITICAL: Clamp to Int.MAX_VALUE to prevent overflow when converting Long to Int
            // AtomicInteger stores Int, but nextDelay could exceed Int.MAX (2.1B ms ≈ 24 days)
            val clampedDelay = minOf(nextDelay, Int.MAX_VALUE.toLong())
            reconnectDelay.set(clampedDelay.toInt())

            if (serverUrl != null) {
                doConnect()
            }
        }
    }

    // -------------------------------------------------------------------------
    // Message framing
    // -------------------------------------------------------------------------

    private fun sendRegistration() {
        val identityHashBytes = hexStringToBytes(myIdentityHash ?: return)
        val frame = buildFrame(RelayMessageType.REGISTER, identityHashBytes, ByteArray(0))
        webSocket?.send(frame.toByteString())
    }

    private fun drainQueue() {
        drainJob?.cancel()
        drainJob = scope.launch {
            while (sendQueue.isNotEmpty() && isConnected.get()) {
                val msg = sendQueue.peek() ?: break
                val sent = withContext(Dispatchers.Main) {
                    sendRaw(msg.recipientHash, msg.sealedContent)
                }
                if (sent) {
                    sendQueue.poll()
                } else {
                    delay(500)
                }
            }
        }
    }

    private fun sendRaw(recipientHash: ByteArray, content: ByteArray): Boolean {
        val ws = webSocket ?: return false
        val frame = buildFrame(RelayMessageType.SEND, recipientHash, content)
        return ws.send(frame.toByteString())
    }

    private fun buildFrame(
        type: RelayMessageType,
        recipient: ByteArray,
        content: ByteArray,
    ): ByteArray {
        // [1 type][32 recipient hash][4 content length][content]
        val buf = ByteArray(1 + 32 + 4 + content.size)
        buf[0] = type.code
        val recipientPadded = recipient.copyOf(32)
        recipientPadded.copyInto(buf, 1)
        buf[33] = (content.size shr 24).toByte()
        buf[34] = (content.size shr 16).toByte()
        buf[35] = (content.size shr 8).toByte()
        buf[36] = content.size.toByte()
        content.copyInto(buf, 37)
        return buf
    }

    private fun handleIncomingBytes(bytes: ByteArray) {
        if (bytes.isEmpty()) return

        when (bytes[0]) {
            RelayMessageType.RECEIVE.code -> {
                if (bytes.size < 37) return
                val senderHash = bytes.copyOfRange(1, 33)
                val contentLen = ((bytes[33].toInt() and 0xFF) shl 24) or
                        ((bytes[34].toInt() and 0xFF) shl 16) or
                        ((bytes[35].toInt() and 0xFF) shl 8) or
                        (bytes[36].toInt() and 0xFF)
                if (bytes.size < 37 + contentLen) return
                val content = bytes.copyOfRange(37, 37 + contentLen)
                onMessageReceived?.invoke(senderHash, content)
            }
            RelayMessageType.CHALLENGE.code -> {
                handleChallenge(bytes.copyOfRange(1, bytes.size))
            }
            RelayMessageType.ACK.code -> {
                // Message acknowledged by relay
                Log.d(TAG, "Message ACK received")
            }
            RelayMessageType.PONG.code -> {
                // Pong received
            }
            RelayMessageType.ERROR.code -> {
                val errorMsg = String(bytes.copyOfRange(1, bytes.size), Charsets.UTF_8)
                Log.e(TAG, "Relay error: $errorMsg")
                onError?.invoke(RuntimeException("Relay error: $errorMsg"))
            }
            else -> Log.w(TAG, "Unknown message type: ${bytes[0]}")
        }
    }

    private fun handleChallenge(challengeBytes: ByteArray) {
        // PRODUCTION: Challenge-response authentication with Dilithium5
        // Server sends nonce, we MUST sign with our Dilithium private key
        // This prevents identity spoofing and ensures only holders of the private key can authenticate

        try {
            // Get our identity from IdentityManager
            val identity = IdentityManager.getIdentity()
                ?: run {
                    Log.e(TAG, "No identity available for challenge response")
                    return
                }

            // Sign challenge with Dilithium5 private key (4864-byte signature)
            val signature = NexusCrypto.dilithiumSign(
                identity.dilithiumPrivateKey,
                challengeBytes
            )

            // Build response frame with signature (not echoed nonce!)
            val response = buildFrame(
                RelayMessageType.CHALLENGE_RESPONSE,
                ByteArray(32),
                signature,
            )
            webSocket?.send(response.toByteString())

            Log.d(TAG, "Challenge signed and sent (signature length: ${signature.size})")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to sign challenge: ${e.message}", e)
        }
    }

    // -------------------------------------------------------------------------
    // Utilities
    // -------------------------------------------------------------------------

    private fun hexStringToBytes(hex: String): ByteArray {
        val clean = hex.replace(":", "").replace(" ", "")
        return ByteArray(clean.length / 2) { i ->
            clean.substring(i * 2, i * 2 + 2).toInt(16).toByte()
        }
    }
}
