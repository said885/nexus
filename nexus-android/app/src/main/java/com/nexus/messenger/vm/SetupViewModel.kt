package com.nexus.messenger.vm

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.nexus.messenger.crypto.NexusCrypto
import com.nexus.messenger.crypto.NexusIdentity
import com.nexus.messenger.crypto.SecureKeyStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

val KEY_SETUP_COMPLETE = booleanPreferencesKey("setup_complete")
val KEY_RELAY_URL = stringPreferencesKey("relay_url")
val KEY_IDENTITY_FINGERPRINT = stringPreferencesKey("identity_fingerprint")

sealed class SetupState {
    object Idle : SetupState()
    object GeneratingIdentity : SetupState()
    data class IdentityGenerated(val fingerprint: String, val identityHashHex: String) : SetupState()
    object SettingUpBiometric : SetupState()
    object ConfiguringRelay : SetupState()
    object Complete : SetupState()
    data class Error(val message: String) : SetupState()
}

class SetupViewModel(
    private val keyStore: SecureKeyStore,
    private val dataStore: DataStore<Preferences>,
) : ViewModel() {

    private val _state = MutableStateFlow<SetupState>(SetupState.Idle)
    val state: StateFlow<SetupState> = _state.asStateFlow()

    private val _setupComplete = MutableStateFlow(false)
    val setupComplete: StateFlow<Boolean> = _setupComplete.asStateFlow()

    init {
        viewModelScope.launch {
            val complete = dataStore.data.map { it[KEY_SETUP_COMPLETE] ?: false }.first()
            _setupComplete.value = complete
        }
    }

    fun generateIdentity() {
        viewModelScope.launch {
            _state.value = SetupState.GeneratingIdentity
            try {
                val identity = withContext(Dispatchers.Default) {
                    keyStore.generateAndStoreIdentity()
                }
                val hash = NexusCrypto.identityHash(identity.dilithiumPublicKey)
                val fingerprint = hash.take(8).joinToString(" ") { "%02X".format(it) }
                val hashHex = hash.joinToString("") { "%02x".format(it) }

                // Store fingerprint in DataStore for display
                dataStore.edit { prefs ->
                    prefs[KEY_IDENTITY_FINGERPRINT] = fingerprint
                }

                _state.value = SetupState.IdentityGenerated(fingerprint, hashHex)
            } catch (e: Exception) {
                _state.value = SetupState.Error("Failed to generate identity: ${e.message}")
            }
        }
    }

    fun proceedToBiometric() {
        _state.value = SetupState.SettingUpBiometric
    }

    fun proceedToRelayConfig() {
        _state.value = SetupState.ConfiguringRelay
    }

    fun saveRelayUrl(url: String) {
        viewModelScope.launch {
            dataStore.edit { prefs ->
                prefs[KEY_RELAY_URL] = url.trim()
            }
        }
    }

    fun completeSetup() {
        viewModelScope.launch {
            dataStore.edit { prefs ->
                prefs[KEY_SETUP_COMPLETE] = true
            }
            _state.value = SetupState.Complete
            _setupComplete.value = true
        }
    }

    fun isSetupComplete(): Boolean = _setupComplete.value

    fun hasIdentity(): Boolean = keyStore.hasIdentity()

    fun getStoredFingerprint(): String? = keyStore.getIdentityFingerprint()
}
