package com.nexus.messenger.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Biometric authentication manager for NEXUS Messenger
 * Provides hardware-backed biometric authentication with fallback to device credentials
 */
class BiometricAuthManager(private val context: Context) {

    companion object {
        private const val KEYSTORE_ALIAS = "nexus_biometric_key"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val IV_SIZE = 12
        private const val TAG_SIZE = 128
    }

    /**
     * Check if biometric authentication is available
     */
    fun canAuthenticate(): BiometricStatus {
        val biometricManager = BiometricManager.from(context)
        return when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> BiometricStatus.Available
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE -> BiometricStatus.NoHardware
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE -> BiometricStatus.HardwareUnavailable
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricStatus.NoneEnrolled
            BiometricManager.BIOMETRIC_ERROR_SECURITY_UPDATE_REQUIRED -> BiometricStatus.SecurityUpdateRequired
            else -> BiometricStatus.Unknown
        }
    }

    /**
     * Show biometric prompt for authentication
     */
    fun authenticate(
        activity: FragmentActivity,
        title: String = "Authenticate",
        subtitle: String = "Verify your identity to access NEXUS",
        description: String = "Place your finger on the sensor or look at the camera",
        onSuccess: () -> Unit,
        onError: (String) -> Unit,
        onFailed: () -> Unit
    ) {
        val executor = ContextCompat.getMainExecutor(activity)

        val callback = object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                super.onAuthenticationSucceeded(result)
                onSuccess()
            }

            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                super.onAuthenticationError(errorCode, errString)
                onError(errString.toString())
            }

            override fun onAuthenticationFailed() {
                super.onAuthenticationFailed()
                onFailed()
            }
        }

        val biometricPrompt = BiometricPrompt(activity, executor, callback)

        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setDescription(description)
            .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            .setNegativeButtonText("Cancel")
            .build()

        biometricPrompt.authenticate(promptInfo)
    }

    /**
     * Show biometric prompt with device credential fallback
     */
    fun authenticateWithFallback(
        activity: FragmentActivity,
        title: String = "Authenticate",
        subtitle: String = "Verify your identity",
        onSuccess: () -> Unit,
        onError: (String) -> Unit,
        onFailed: () -> Unit
    ) {
        val executor = ContextCompat.getMainExecutor(activity)

        val callback = object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                super.onAuthenticationSucceeded(result)
                onSuccess()
            }

            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                super.onAuthenticationError(errorCode, errString)
                onError(errString.toString())
            }

            override fun onAuthenticationFailed() {
                super.onAuthenticationFailed()
                onFailed()
            }
        }

        val biometricPrompt = BiometricPrompt(activity, executor, callback)

        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setAllowedAuthenticators(
                BiometricManager.Authenticators.BIOMETRIC_STRONG or
                BiometricManager.Authenticators.DEVICE_CREDENTIAL
            )
            .build()

        biometricPrompt.authenticate(promptInfo)
    }

    /**
     * Generate or get the biometric-protected key
     */
    private fun getOrCreateKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)

        // Check if key exists
        if (keyStore.containsAlias(KEYSTORE_ALIAS)) {
            return keyStore.getKey(KEYSTORE_ALIAS, null) as SecretKey
        }

        // Generate new key
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEYSTORE
        )

        val keySpec = KeyGenParameterSpec.Builder(
            KEYSTORE_ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(true)
            .setInvalidatedByBiometricEnrollment(true)
            .build()

        keyGenerator.init(keySpec)
        return keyGenerator.generateKey()
    }

    /**
     * Encrypt data with biometric protection
     */
    fun encrypt(data: ByteArray): EncryptedData {
        val key = getOrCreateKey()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)

        val iv = cipher.iv
        val encrypted = cipher.doFinal(data)

        return EncryptedData(encrypted, iv)
    }

    /**
     * Decrypt data (requires biometric authentication)
     */
    fun decrypt(encryptedData: EncryptedData): ByteArray {
        val key = getOrCreateKey()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        val spec = GCMParameterSpec(TAG_SIZE, encryptedData.iv)
        cipher.init(Cipher.DECRYPT_MODE, key, spec)

        return cipher.doFinal(encryptedData.data)
    }

    /**
     * Get cipher for biometric-bound decryption
     */
    fun getDecryptCipher(encryptedData: EncryptedData): Cipher {
        val key = getOrCreateKey()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        val spec = GCMParameterSpec(TAG_SIZE, encryptedData.iv)
        cipher.init(Cipher.DECRYPT_MODE, key, spec)
        return cipher
    }

    /**
     * Delete the biometric key
     */
    fun deleteKey() {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)
        keyStore.deleteEntry(KEYSTORE_ALIAS)
    }

    /**
     * Check if biometric key exists
     */
    fun hasKey(): Boolean {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)
        return keyStore.containsAlias(KEYSTORE_ALIAS)
    }
}

/**
 * Biometric availability status
 */
enum class BiometricStatus {
    Available,
    NoHardware,
    HardwareUnavailable,
    NoneEnrolled,
    SecurityUpdateRequired,
    Unknown
}

/**
 * Encrypted data container
 */
data class EncryptedData(
    val data: ByteArray,
    val iv: ByteArray
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as EncryptedData

        if (!data.contentEquals(other.data)) return false
        if (!iv.contentEquals(other.iv)) return false

        return true
    }

    override fun hashCode(): Int {
        var result = data.contentHashCode()
        result = 31 * result + iv.contentHashCode()
        return result
    }
}
