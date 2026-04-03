package com.nexus.messenger.di

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.preferencesDataStore
import com.nexus.messenger.crypto.SecureKeyStore
import com.nexus.messenger.data.NexusDatabase
import com.nexus.messenger.network.RelayClient
import com.nexus.messenger.vm.ChatViewModel
import com.nexus.messenger.vm.ConversationsViewModel
import com.nexus.messenger.vm.SetupViewModel
import org.koin.android.ext.koin.androidContext
import org.koin.androidx.viewmodel.dsl.viewModel
import org.koin.dsl.module

val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "nexus_prefs")

val appModule = module {

    single<SecureKeyStore> {
        SecureKeyStore(androidContext())
    }

    single<NexusDatabase> {
        // Database key derived from a constant + device-specific info;
        // in production this should be derived from the Keystore-protected identity.
        val dbKey = "nexus_db_key_placeholder_v1" // TODO: derive from KeyStore
        NexusDatabase(androidContext(), dbKey)
    }

    single<RelayClient> {
        RelayClient()
    }

    single<DataStore<Preferences>> {
        androidContext().dataStore
    }

    viewModel {
        SetupViewModel(get(), get())
    }

    viewModel { (conversationId: Long, participantHash: String) ->
        ChatViewModel(conversationId, participantHash, get(), get(), get(), get())
    }

    viewModel {
        ConversationsViewModel(get(), get(), get())
    }
}
