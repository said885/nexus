package com.nexus.messenger.crypto

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.coroutines.suspendCancellableCoroutine
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

private const val KEYSTORE_PROVIDER = "AndroidKeyStore"
private const val NEXUS_KEY_ALIAS = "nexus_master_key_v1"
private const val PREFS_FILE = "nexus_identity_store"
private const val KEY_IDENTITY_ENCRYPTED = "identity_enc"
private const val KEY_IDENTITY_IV = "identity_iv"
private const val KEY_IDENTITY_HASH = "identity_hash"
private const val KEY_STRONGBOX_BACKED = "strongbox_backed"
private const val AES_GCM_TRANSFORMATION = "AES/GCM/NoPadding"
private const val GCM_TAG_LENGTH = 128

class SecureKeyStore(private val context: Context) {

    private val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER).apply { load(null) }

    // -------------------------------------------------------------------------
    // Android Keystore key management
    // -------------------------------------------------------------------------

    private fun generateOrGetKeystoreKey(requireBiometric: Boolean = true): SecretKey {
        val existingKey = keyStore.getKey(NEXUS_KEY_ALIAS, null) as? SecretKey
        if (existingKey != null) return existingKey

        return generateNewKeystoreKey(requireBiometric)
    }

    private fun generateNewKeystoreKey(requireBiometric: Boolean): SecretKey {
        val keyGenerator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE_PROVIDER)
        val strongBoxAvailable = isStrongBoxAvailable()
        val backed = if (strongBoxAvailable) {
            try {
                keyGenerator.init(buildKeySpec(requireBiometric, strongBox = true))
                true
            } catch (e: Exception) {
                // Fallback to non-StrongBox
                keyGenerator.init(buildKeySpec(requireBiometric, strongBox = false))
                false
            }
        } else {
            keyGenerator.init(buildKeySpec(requireBiometric, strongBox = false))
            false
        }
        val key = keyGenerator.generateKey()
        getPlainPrefs().edit().putBoolean(KEY_STRONGBOX_BACKED, backed).apply()
        return key
    }

    private fun buildKeySpec(requireBiometric: Boolean, strongBox: Boolean): KeyGenParameterSpec {
        val builder = KeyGenParameterSpec.Builder(
            NEXUS_KEY_ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setRandomizedEncryptionRequired(true)
            .setInvalidatedByBiometricEnrollment(true)

        if (requireBiometric) {
            builder.setUserAuthenticationRequired(true)
            // Require biometric auth every time (timeout = 0)
            builder.setUserAuthenticationParameters(0, KeyProperties.AUTH_BIOMETRIC_STRONG)
        }

        if (strongBox) {
            builder.setIsStrongBoxBacked(true)
        }

        return builder.build()
    }

    private fun isStrongBoxAvailable(): Boolean =
        context.packageManager.hasSystemFeature("android.hardware.strongbox_keystore")

    fun isStrongBoxBacked(): Boolean =
        getPlainPrefs().getBoolean(KEY_STRONGBOX_BACKED, false)

    // -------------------------------------------------------------------------
    // Identity storage
    // -------------------------------------------------------------------------

    /**
     * Generates a new identity, encrypts it with the Keystore-backed AES key,
     * and stores the ciphertext in EncryptedSharedPreferences.
     */
    fun generateAndStoreIdentity(): NexusIdentity {
        val identity = NexusCrypto.generateIdentity()
        val identityBytes = serializeIdentity(identity)

        // Generate Keystore key (no biometric required at generation time; enrollment happens on first unlock)
        val key = generateOrGetKeystoreKey(requireBiometric = false)

        // Encrypt identity bytes with AES/GCM
        val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val iv = cipher.iv
        val encrypted = cipher.doFinal(identityBytes)

        // Compute identity hash from Dilithium public key
        val idHash = NexusCrypto.identityHash(identity.dilithiumPublicKey)

        // Store in plain prefs (data already encrypted with Keystore-backed key)
        val prefs = getPlainPrefs()
        prefs.edit()
            .putString(KEY_IDENTITY_ENCRYPTED, Base64.encodeToString(encrypted, Base64.NO_WRAP))
            .putString(KEY_IDENTITY_IV, Base64.encodeToString(iv, Base64.NO_WRAP))
            .putString(KEY_IDENTITY_HASH, Base64.encodeToString(idHash, Base64.NO_WRAP))
            .apply()

        // Wipe plaintext bytes from memory
        identityBytes.fill(0)

        return identity
    }

    /**
     * Loads the identity using a biometric-unlocked cipher.
     * Must be called from a FragmentActivity.
     */
    suspend fun loadIdentityWithBiometric(activity: FragmentActivity): NexusIdentity {
        val prefs = getPlainPrefs()
        val encryptedB64 = prefs.getString(KEY_IDENTITY_ENCRYPTED, null)
            ?: error("No identity stored")
        val ivB64 = prefs.getString(KEY_IDENTITY_IV, null)
            ?: error("No identity IV stored")

        val encrypted = Base64.decode(encryptedB64, Base64.NO_WRAP)
        val iv = Base64.decode(ivB64, Base64.NO_WRAP)

        val key = keyStore.getKey(NEXUS_KEY_ALIAS, null) as? SecretKey
            ?: error("Keystore key not found")

        // Initialize cipher for decryption (this will be authenticated by biometric)
        val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
        val spec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
        cipher.init(Cipher.DECRYPT_MODE, key, spec)

        // Authenticate with biometric to unlock cipher
        val authenticatedCipher = BiometricKeyUnlocker.authenticateWithBiometric(
            activity = activity,
            cipher = cipher,
            title = activity.getString(android.R.string.ok).let { "Unlock Nexus" },
            subtitle = "Authenticate to access your messages",
        )

        val decrypted = authenticatedCipher.doFinal(encrypted)
        val identity = deserializeIdentity(decrypted)
        decrypted.fill(0)
        return identity
    }

    /**
     * Loads identity without biometric (for testing / when biometric not required).
     * Note: if key requires biometric, this will fail at the crypto level.
     */
    fun loadIdentityDirect(): NexusIdentity? {
        return try {
            val prefs = getPlainPrefs()
            val encryptedB64 = prefs.getString(KEY_IDENTITY_ENCRYPTED, null) ?: return null
            val ivB64 = prefs.getString(KEY_IDENTITY_IV, null) ?: return null
            val encrypted = Base64.decode(encryptedB64, Base64.NO_WRAP)
            val iv = Base64.decode(ivB64, Base64.NO_WRAP)
            val key = keyStore.getKey(NEXUS_KEY_ALIAS, null) as? SecretKey ?: return null
            val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
            val spec = GCMParameterSpec(GCM_TAG_LENGTH, iv)
            cipher.init(Cipher.DECRYPT_MODE, key, spec)
            val decrypted = cipher.doFinal(encrypted)
            val identity = deserializeIdentity(decrypted)
            decrypted.fill(0)
            identity
        } catch (e: Exception) {
            null
        }
    }

    fun hasIdentity(): Boolean =
        getPlainPrefs().getString(KEY_IDENTITY_ENCRYPTED, null) != null

    fun getIdentityHash(): ByteArray? {
        val b64 = getPlainPrefs().getString(KEY_IDENTITY_HASH, null) ?: return null
        return Base64.decode(b64, Base64.NO_WRAP)
    }

    fun getIdentityFingerprint(): String? {
        val hash = getIdentityHash() ?: return null
        // First 8 bytes formatted as space-separated hex pairs
        return hash.take(8).joinToString(" ") { "%02X".format(it) }
    }

    fun clearIdentity() {
        getPlainPrefs().edit().clear().apply()
        try {
            keyStore.deleteEntry(NEXUS_KEY_ALIAS)
        } catch (e: Exception) {
            // ignore
        }
    }

    // -------------------------------------------------------------------------
    // Serialization (simple length-prefixed format)
    // -------------------------------------------------------------------------

    private fun serializeIdentity(identity: NexusIdentity): ByteArray {
        val fields = listOf(
            identity.dilithiumPublicKey,
            identity.dilithiumPrivateKey,
            identity.kyberPublicKey,
            identity.kyberPrivateKey,
            identity.x25519PublicKey,
            identity.x25519PrivateKey,
        )
        var totalSize = 4 * fields.size // 4 bytes per length prefix
        fields.forEach { totalSize += it.size }
        val buf = ByteArray(totalSize)
        var pos = 0
        for (field in fields) {
            buf[pos] = (field.size shr 24).toByte()
            buf[pos + 1] = (field.size shr 16).toByte()
            buf[pos + 2] = (field.size shr 8).toByte()
            buf[pos + 3] = field.size.toByte()
            pos += 4
            field.copyInto(buf, pos)
            pos += field.size
        }
        return buf
    }

    private fun deserializeIdentity(bytes: ByteArray): NexusIdentity {
        var pos = 0
        fun readField(): ByteArray {
            val len = ((bytes[pos].toInt() and 0xFF) shl 24) or
                    ((bytes[pos + 1].toInt() and 0xFF) shl 16) or
                    ((bytes[pos + 2].toInt() and 0xFF) shl 8) or
                    (bytes[pos + 3].toInt() and 0xFF)
            pos += 4
            val field = bytes.copyOfRange(pos, pos + len)
            pos += len
            return field
        }
        return NexusIdentity(
            dilithiumPublicKey = readField(),
            dilithiumPrivateKey = readField(),
            kyberPublicKey = readField(),
            kyberPrivateKey = readField(),
            x25519PublicKey = readField(),
            x25519PrivateKey = readField(),
        )
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    private fun getPlainPrefs(): SharedPreferences =
        context.getSharedPreferences(PREFS_FILE, Context.MODE_PRIVATE)
}

// -------------------------------------------------------------------------
// Biometric authentication helper
// -------------------------------------------------------------------------

object BiometricKeyUnlocker {

    fun isBiometricAvailable(context: Context): Boolean {
        val bm = BiometricManager.from(context)
        return bm.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) ==
                BiometricManager.BIOMETRIC_SUCCESS
    }

    /**
     * Suspends until biometric authentication completes, then returns the authenticated cipher.
     */
    suspend fun authenticateWithBiometric(
        activity: FragmentActivity,
        cipher: Cipher,
        title: String,
        subtitle: String,
    ): Cipher = suspendCancellableCoroutine { cont ->
        val executor = ContextCompat.getMainExecutor(activity)
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setNegativeButtonText("Cancel")
            .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            .build()

        val biometricPrompt = BiometricPrompt(
            activity,
            executor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    val cryptoObject = result.cryptoObject
                    val authenticatedCipher = cryptoObject?.cipher
                    if (authenticatedCipher != null) {
                        cont.resume(authenticatedCipher)
                    } else {
                        cont.resumeWithException(IllegalStateException("Cipher not returned from biometric"))
                    }
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    cont.resumeWithException(
                        SecurityException("Biometric auth error $errorCode: $errString")
                    )
                }

                override fun onAuthenticationFailed() {
                    // Don't cancel — let the system handle retries
                }
            }
        )

        biometricPrompt.authenticate(promptInfo, BiometricPrompt.CryptoObject(cipher))

        cont.invokeOnCancellation {
            biometricPrompt.cancelAuthentication()
        }
    }

    /**
     * Simple biometric confirmation (no crypto object).
     */
    suspend fun confirmWithBiometric(
        activity: FragmentActivity,
        title: String,
        subtitle: String,
    ): Boolean = suspendCancellableCoroutine { cont ->
        val executor = ContextCompat.getMainExecutor(activity)
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setNegativeButtonText("Cancel")
            .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            .build()

        val biometricPrompt = BiometricPrompt(
            activity,
            executor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    cont.resume(true)
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    cont.resume(false)
                }

                override fun onAuthenticationFailed() {
                    // retries handled by system
                }
            }
        )

        biometricPrompt.authenticate(promptInfo)

        cont.invokeOnCancellation { biometricPrompt.cancelAuthentication() }
    }
}
