package com.nexus.messenger.vm

import android.util.Base64
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.nexus.messenger.crypto.EncryptedMessage
import com.nexus.messenger.crypto.MessageHeader
import com.nexus.messenger.crypto.NexusCrypto
import com.nexus.messenger.crypto.RatchetState
import com.nexus.messenger.crypto.SecureKeyStore
import com.nexus.messenger.data.ConversationEntity
import com.nexus.messenger.data.MessageEntity
import com.nexus.messenger.data.NexusDatabase
import com.nexus.messenger.network.RelayClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private const val TAG = "ChatViewModel"

sealed class MessageStatus {
    object Sending : MessageStatus()
    object Sent : MessageStatus()
    object Delivered : MessageStatus()
    object Read : MessageStatus()
    object Failed : MessageStatus()
}

data class DisplayMessage(
    val id: Long,
    val content: String,    // Decrypted plaintext
    val timestamp: Long,
    val isMine: Boolean,
    val delivered: Boolean,
    val read: Boolean,
    val autoDeleteAt: Long?,
)

@Serializable
data class SerializableRatchetState(
    val rootKey: String,           // Base64
    val sendChainKey: String?,
    val recvChainKey: String?,
    val sendMsgN: Int,
    val recvMsgN: Int,
    val prevSendCount: Int,
    val dhSendPriv: String,
    val dhSendPub: String,
    val dhRemotePub: String?,
    val skippedKeys: Map<String, String>,  // "pubHex:msgN" -> Base64(msgKey)
)

@Serializable
data class WireMessage(
    val dhPublic: String,      // Base64
    val msgN: Int,
    val prevChainLen: Int,
    val ciphertext: String,    // Base64
)

class ChatViewModel(
    private val conversationId: Long,
    private val participantHash: String,
    private val keyStore: SecureKeyStore,
    private val database: NexusDatabase,
    private val relayClient: RelayClient,
    private val dataStore: DataStore<Preferences>,
) : ViewModel() {

    private val _messages = MutableStateFlow<List<DisplayMessage>>(emptyList())
    val messages: StateFlow<List<DisplayMessage>> = _messages.asStateFlow()

    private val _isSessionSecure = MutableStateFlow(false)
    val isSessionSecure: StateFlow<Boolean> = _isSessionSecure.asStateFlow()

    private val _isVerified = MutableStateFlow(false)
    val isVerified: StateFlow<Boolean> = _isVerified.asStateFlow()

    private val _isSending = MutableStateFlow(false)
    val isSending: StateFlow<Boolean> = _isSending.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private var ratchetState: RatchetState? = null
    private var myIdentityHashBytes: ByteArray? = null

    private val json = Json { ignoreUnknownKeys = true }

    init {
        loadMessages()
        loadSessionState()
        setupMessageReceiver()
    }

    // -------------------------------------------------------------------------
    // Initialization
    // -------------------------------------------------------------------------

    private fun loadMessages() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val rawMessages = database.getMessagesForConversation(conversationId)
                val displayed = rawMessages.mapNotNull { entity ->
                    decryptMessageEntity(entity)
                }
                _messages.value = displayed
            } catch (e: Exception) {
                Log.e(TAG, "Failed to load messages", e)
                _error.value = "Failed to load messages"
            }
        }
    }

    private fun loadSessionState() {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val conv = database.getConversationByHash(participantHash)
                _isVerified.value = conv?.verified ?: false

                // Load ratchet state if it exists
                conv?.ratchetStateSerialized?.let { serialized ->
                    ratchetState = deserializeRatchetState(serialized)
                    _isSessionSecure.value = true
                }

                // Get my identity hash
                myIdentityHashBytes = keyStore.getIdentityHash()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to load session state", e)
            }
        }
    }

    private fun setupMessageReceiver() {
        relayClient.onMessageReceived = { senderHashBytes, content ->
            val senderHex = senderHashBytes.joinToString("") { "%02x".format(it) }
            if (senderHex == participantHash) {
                viewModelScope.launch(Dispatchers.IO) {
                    handleIncomingMessage(content)
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Send message
    // -------------------------------------------------------------------------

    fun sendMessage(plaintext: String) {
        if (plaintext.isBlank()) return
        viewModelScope.launch(Dispatchers.IO) {
            _isSending.value = true
            try {
                val state = ratchetState ?: run {
                    // No ratchet state: initiate X3DH (simplified — in real implementation
                    // we'd fetch the pre-key bundle from the relay server)
                    Log.w(TAG, "No ratchet state; cannot send (need to complete key exchange)")
                    _error.value = "Session not established. Waiting for key exchange."
                    return@launch
                }

                val plaintextBytes = plaintext.toByteArray(Charsets.UTF_8)
                val myHashBytes = myIdentityHashBytes ?: ByteArray(32)
                val assocData = myHashBytes + participantHash.hexToBytes()

                val encrypted = NexusCrypto.ratchetEncrypt(state, plaintextBytes, assocData)
                saveRatchetState(state)

                // Serialize to wire format
                val wire = WireMessage(
                    dhPublic = Base64.encodeToString(encrypted.header.dhPublic, Base64.NO_WRAP),
                    msgN = encrypted.header.msgN,
                    prevChainLen = encrypted.header.prevChainLen,
                    ciphertext = Base64.encodeToString(encrypted.ciphertext, Base64.NO_WRAP),
                )
                val wireBytes = json.encodeToString(wire).toByteArray(Charsets.UTF_8)

                // Store locally (encrypted with a local key)
                val (encContent, encIv) = encryptForStorage(plaintext)
                val msgId = database.insertMessage(
                    MessageEntity(
                        conversationId = conversationId,
                        senderHash = myHashBytes.joinToString("") { "%02x".format(it) },
                        content = encContent,
                        contentIv = encIv,
                        timestamp = System.currentTimeMillis(),
                        delivered = false,
                        read = true,
                        autoDeleteAt = null,
                        isMine = true,
                    )
                )

                // Update conversation preview
                database.insertOrUpdateConversation(
                    ConversationEntity(
                        participantHash = participantHash,
                        participantName = null,
                        lastMessagePreview = encContent,
                        lastMessageTime = System.currentTimeMillis(),
                        unreadCount = 0,
                        ratchetStateSerialized = serializeRatchetState(state),
                    )
                )

                // Add to display immediately (optimistic)
                val display = DisplayMessage(
                    id = msgId,
                    content = plaintext,
                    timestamp = System.currentTimeMillis(),
                    isMine = true,
                    delivered = false,
                    read = true,
                    autoDeleteAt = null,
                )
                _messages.value = _messages.value + display

                // Send over relay
                val recipientHashBytes = participantHash.hexToBytes()
                relayClient.sendMessage(recipientHashBytes, wireBytes)

            } catch (e: Exception) {
                Log.e(TAG, "Failed to send message", e)
                _error.value = "Failed to send: ${e.message}"
            } finally {
                _isSending.value = false
            }
        }
    }

    // -------------------------------------------------------------------------
    // Receive message
    // -------------------------------------------------------------------------

    private suspend fun handleIncomingMessage(wireBytes: ByteArray) {
        try {
            val wireJson = String(wireBytes, Charsets.UTF_8)
            val wire = json.decodeFromString<WireMessage>(wireJson)

            val state = ratchetState ?: run {
                Log.w(TAG, "Received message but no ratchet state")
                return
            }

            val encMsg = EncryptedMessage(
                header = MessageHeader(
                    dhPublic = Base64.decode(wire.dhPublic, Base64.NO_WRAP),
                    msgN = wire.msgN,
                    prevChainLen = wire.prevChainLen,
                ),
                ciphertext = Base64.decode(wire.ciphertext, Base64.NO_WRAP),
            )

            val myHashBytes = myIdentityHashBytes ?: ByteArray(32)
            val assocData = participantHash.hexToBytes() + myHashBytes
            val plaintext = NexusCrypto.ratchetDecrypt(state, encMsg, assocData)
            saveRatchetState(state)

            val plaintextStr = String(plaintext, Charsets.UTF_8)
            NexusCrypto.wipe(plaintext)

            val (encContent, encIv) = encryptForStorage(plaintextStr)
            val msgId = database.insertMessage(
                MessageEntity(
                    conversationId = conversationId,
                    senderHash = participantHash,
                    content = encContent,
                    contentIv = encIv,
                    timestamp = System.currentTimeMillis(),
                    delivered = true,
                    read = false,
                    autoDeleteAt = null,
                    isMine = false,
                )
            )

            val display = DisplayMessage(
                id = msgId,
                content = plaintextStr,
                timestamp = System.currentTimeMillis(),
                isMine = false,
                delivered = true,
                read = false,
                autoDeleteAt = null,
            )
            _messages.value = _messages.value + display

        } catch (e: Exception) {
            Log.e(TAG, "Failed to process incoming message", e)
        }
    }

    // -------------------------------------------------------------------------
    // Message actions
    // -------------------------------------------------------------------------

    fun deleteMessage(messageId: Long) {
        viewModelScope.launch(Dispatchers.IO) {
            database.deleteMessage(messageId)
            _messages.value = _messages.value.filter { it.id != messageId }
        }
    }

    fun setMessageTimer(messageId: Long, deleteAfterMs: Long?) {
        viewModelScope.launch(Dispatchers.IO) {
            val deleteAt = if (deleteAfterMs != null) System.currentTimeMillis() + deleteAfterMs else null
            database.setMessageAutoDelete(messageId, deleteAt)
            _messages.value = _messages.value.map { msg ->
                if (msg.id == messageId) msg.copy(autoDeleteAt = deleteAt) else msg
            }
        }
    }

    fun markAsRead(messageId: Long) {
        viewModelScope.launch(Dispatchers.IO) {
            database.markMessageRead(messageId)
        }
    }

    fun clearError() {
        _error.value = null
    }

    // -------------------------------------------------------------------------
    // Key exchange helpers
    // -------------------------------------------------------------------------

    /**
     * Called when we receive a pre-key bundle from a new contact and want
     * to initiate the session as sender.
     */
    fun initiateSession(
        remoteKyberPubKey: ByteArray,
        remoteSignedPreKey: ByteArray,
        remoteSignedPreKeySignature: ByteArray,
        remoteX25519PreKey: ByteArray?,
    ) {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val myIdentity = keyStore.loadIdentityDirect() ?: run {
                    _error.value = "Could not load identity"
                    return@launch
                }

                val bundle = com.nexus.messenger.crypto.PreKeyBundle(
                    identityKey = remoteKyberPubKey,
                    signedPreKey = remoteSignedPreKey,
                    signedPreKeySignature = remoteSignedPreKeySignature,
                    oneTimePreKey = remoteX25519PreKey,
                    oneTimePreKeyId = null,
                )

                val (masterSecret, _) = NexusCrypto.x3dhSend(myIdentity, bundle)
                val state = NexusCrypto.initRatchetSender(masterSecret, remoteSignedPreKey)
                NexusCrypto.wipe(masterSecret)

                ratchetState = state
                saveRatchetState(state)
                _isSessionSecure.value = true

            } catch (e: Exception) {
                Log.e(TAG, "Failed to initiate session", e)
                _error.value = "Failed to establish session: ${e.message}"
            }
        }
    }

    // -------------------------------------------------------------------------
    // Storage encryption (message content at rest)
    // -------------------------------------------------------------------------

    private fun encryptForStorage(plaintext: String): Pair<String, String> {
        // Use a derived key for at-rest encryption
        // In production, this key should be derived from the identity key or Keystore
        val key = NexusCrypto.hkdf(
            ikm = "nexus-storage-key".toByteArray(),
            salt = null,
            info = conversationId.toString().toByteArray(),
            length = 32
        )
        val nonce = NexusCrypto.randomBytes(12)
        val ciphertext = NexusCrypto.chachaEncrypt(
            key = key,
            nonce = nonce,
            plaintext = plaintext.toByteArray(Charsets.UTF_8),
            aad = ByteArray(0)
        )
        NexusCrypto.wipe(key)
        return Pair(
            Base64.encodeToString(ciphertext, Base64.NO_WRAP),
            Base64.encodeToString(nonce, Base64.NO_WRAP)
        )
    }

    private fun decryptForStorage(encContent: String, encIv: String): String? {
        return try {
            val key = NexusCrypto.hkdf(
                ikm = "nexus-storage-key".toByteArray(),
                salt = null,
                info = conversationId.toString().toByteArray(),
                length = 32
            )
            val nonce = Base64.decode(encIv, Base64.NO_WRAP)
            val ciphertext = Base64.decode(encContent, Base64.NO_WRAP)
            val plaintext = NexusCrypto.chachaDecrypt(
                key = key,
                nonce = nonce,
                ciphertext = ciphertext,
                aad = ByteArray(0)
            )
            NexusCrypto.wipe(key)
            String(plaintext, Charsets.UTF_8)
        } catch (e: Exception) {
            Log.e(TAG, "Decryption failed", e)
            null
        }
    }

    private fun decryptMessageEntity(entity: MessageEntity): DisplayMessage? {
        val content = decryptForStorage(entity.content, entity.contentIv) ?: return null
        return DisplayMessage(
            id = entity.id,
            content = content,
            timestamp = entity.timestamp,
            isMine = entity.isMine,
            delivered = entity.delivered,
            read = entity.read,
            autoDeleteAt = entity.autoDeleteAt,
        )
    }

    // -------------------------------------------------------------------------
    // Ratchet state serialization
    // -------------------------------------------------------------------------

    private fun serializeRatchetState(state: RatchetState): String {
        val skippedMap = state.skippedKeys.entries.associate { (k, v) ->
            "${k.first}:${k.second}" to Base64.encodeToString(v, Base64.NO_WRAP)
        }
        val serializable = SerializableRatchetState(
            rootKey = Base64.encodeToString(state.rootKey, Base64.NO_WRAP),
            sendChainKey = state.sendChainKey?.let { Base64.encodeToString(it, Base64.NO_WRAP) },
            recvChainKey = state.recvChainKey?.let { Base64.encodeToString(it, Base64.NO_WRAP) },
            sendMsgN = state.sendMsgN,
            recvMsgN = state.recvMsgN,
            prevSendCount = state.prevSendCount,
            dhSendPriv = Base64.encodeToString(state.dhSendPriv, Base64.NO_WRAP),
            dhSendPub = Base64.encodeToString(state.dhSendPub, Base64.NO_WRAP),
            dhRemotePub = state.dhRemotePub?.let { Base64.encodeToString(it, Base64.NO_WRAP) },
            skippedKeys = skippedMap,
        )
        return json.encodeToString(serializable)
    }

    private fun deserializeRatchetState(serialized: String): RatchetState {
        val s = json.decodeFromString<SerializableRatchetState>(serialized)
        val skippedKeys = s.skippedKeys.entries.associate { (k, v) ->
            val parts = k.split(":")
            Pair(parts[0], parts[1].toInt()) to Base64.decode(v, Base64.NO_WRAP)
        }.toMutableMap()
        return RatchetState(
            rootKey = Base64.decode(s.rootKey, Base64.NO_WRAP),
            sendChainKey = s.sendChainKey?.let { Base64.decode(it, Base64.NO_WRAP) },
            recvChainKey = s.recvChainKey?.let { Base64.decode(it, Base64.NO_WRAP) },
            sendMsgN = s.sendMsgN,
            recvMsgN = s.recvMsgN,
            prevSendCount = s.prevSendCount,
            dhSendPriv = Base64.decode(s.dhSendPriv, Base64.NO_WRAP),
            dhSendPub = Base64.decode(s.dhSendPub, Base64.NO_WRAP),
            dhRemotePub = s.dhRemotePub?.let { Base64.decode(it, Base64.NO_WRAP) },
            skippedKeys = skippedKeys,
        )
    }

    private fun saveRatchetState(state: RatchetState) {
        try {
            val serialized = serializeRatchetState(state)
            database.updateRatchetState(participantHash, serialized)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save ratchet state", e)
        }
    }

    // -------------------------------------------------------------------------
    // Utilities
    // -------------------------------------------------------------------------

    private fun String.hexToBytes(): ByteArray {
        val clean = replace(":", "").replace(" ", "")
        return ByteArray(clean.length / 2) { i ->
            clean.substring(i * 2, i * 2 + 2).toInt(16).toByte()
        }
    }
}
