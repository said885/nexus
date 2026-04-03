package com.nexus.messenger.vm

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.nexus.messenger.crypto.SecureKeyStore
import com.nexus.messenger.data.ConversationEntity
import com.nexus.messenger.data.NexusDatabase
import com.nexus.messenger.network.RelayClient
import com.nexus.messenger.security.IntegrityChecker
import com.nexus.messenger.security.ThreatLevel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

class ConversationsViewModel(
    private val keyStore: SecureKeyStore,
    private val database: NexusDatabase,
    private val dataStore: DataStore<Preferences>,
) : ViewModel() {

    private val _conversations = MutableStateFlow<List<ConversationEntity>>(emptyList())
    val conversations: StateFlow<List<ConversationEntity>> = _conversations.asStateFlow()

    private val _isConnected = MutableStateFlow(false)
    val isConnected: StateFlow<Boolean> = _isConnected.asStateFlow()

    private val _threatLevel = MutableStateFlow(ThreatLevel.NONE)
    val threatLevel: StateFlow<ThreatLevel> = _threatLevel.asStateFlow()

    private val _identityFingerprint = MutableStateFlow<String?>(null)
    val identityFingerprint: StateFlow<String?> = _identityFingerprint.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        loadConversations()
        loadIdentityInfo()
        startAutoDeleteJob()
    }

    private fun loadConversations() {
        viewModelScope.launch(Dispatchers.IO) {
            _isLoading.value = true
            try {
                val convs = database.getAllConversations()
                _conversations.value = convs
            } catch (e: Exception) {
                // Log error
            } finally {
                _isLoading.value = false
            }
        }
    }

    private fun loadIdentityInfo() {
        viewModelScope.launch {
            _identityFingerprint.value = keyStore.getIdentityFingerprint()
        }
    }

    fun refreshConversations() {
        loadConversations()
    }

    fun deleteConversation(conversation: ConversationEntity) {
        viewModelScope.launch(Dispatchers.IO) {
            database.deleteConversation(conversation.id)
            _conversations.value = _conversations.value.filter { it.id != conversation.id }
        }
    }

    fun createConversation(participantHash: String, name: String? = null): Long {
        var id = 0L
        viewModelScope.launch(Dispatchers.IO) {
            val existing = database.getConversationByHash(participantHash)
            id = if (existing != null) {
                existing.id
            } else {
                database.insertOrUpdateConversation(
                    ConversationEntity(
                        participantHash = participantHash,
                        participantName = name,
                        lastMessagePreview = null,
                        lastMessageTime = System.currentTimeMillis(),
                        unreadCount = 0,
                        ratchetStateSerialized = null,
                    )
                )
            }
            loadConversations()
        }
        return id
    }

    fun updateThreatLevel(level: ThreatLevel) {
        _threatLevel.value = level
    }

    fun updateConnectionStatus(connected: Boolean) {
        _isConnected.value = connected
    }

    private fun startAutoDeleteJob() {
        viewModelScope.launch(Dispatchers.IO) {
            while (true) {
                try {
                    val deleted = database.deleteExpiredMessages()
                    if (deleted > 0) {
                        loadConversations()
                    }
                } catch (e: Exception) {
                    // Ignore
                }
                delay(60_000L) // Check every minute
            }
        }
    }
}
