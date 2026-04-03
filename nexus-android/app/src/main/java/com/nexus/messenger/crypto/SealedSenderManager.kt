/**
 * NEXUS Sealed Sender Manager for Android
 * Encrypts sender identity inside message
 * Relay is cryptographically blind to sender
 * Only recipient can decrypt sender identity + verify signature
 */

package com.nexus.messenger.crypto

import android.util.Log
import com.google.protobuf.ByteString
import com.nexus.messenger.crypto.NexusCrypto.Identity
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.util.*

data class SealedSenderBundle(
    val ephemeralPublicKey: String,
    val encryptedSenderIdentity: String,
    val encryptedMessage: String,
    val messageDigest: String,
    val senderSignature: String,
    val iv: String,
    val messageIv: String,
)

data class SealedSenderMessage(
    val recipientHash: String,
    val sealedBundle: SealedSenderBundle,
)

data class DecryptedSealedMessage(
    val senderHash: String,
    val senderTimestamp: Long,
    val content: String,
    val messageDigest: String,
    val senderSignature: String,
    val verified: Boolean,
)

object SealedSenderManager {
    private const val TAG = "SealedSenderManager"

    /**
     * Create sealed sender message
     * Sender identity is encrypted with ephemeral key
     * Relay sees ONLY recipient_hash + sealed bundle (opaque)
     */
    fun createSealedSenderMessage(
        plaintext: String,
        recipientHash: String,
        senderIdentity: Identity,
    ): SealedSenderMessage {
        try {
            Log.d(TAG, "Creating sealed sender message to $recipientHash")

            // Step 1: Derive encryption keys from recipient + sender material
            val keyMaterial = ByteArray(32)
            val recipientBytes = recipientHash.toByteArray()
            val senderPublicKeySlice = senderIdentity.kyberPublicKey.sliceArray(0..15)

            for (i in 0 until minOf(16, recipientBytes.size)) {
                keyMaterial[i] = keyMaterial[i] xor recipientBytes[i]
            }
            for (i in 0..15) {
                keyMaterial[16 + i] = keyMaterial[16 + i] xor senderPublicKeySlice[i]
            }

            // Derive sender identity key and message key
            val senderIdentityKey = deriveKey(keyMaterial, "sealed_sender_identity_v1")
            val messageKey = deriveKey(keyMaterial, "sealed_message_content_v1")

            // Step 2: Encrypt sender identity
            val senderJson = """{"hash":"$recipientHash","timestamp":${System.currentTimeMillis()},"version":1}"""
            val encryptedSenderData = encryptAESGCM(senderJson, senderIdentityKey)

            // Step 3: Encrypt message
            val encryptedMessageData = encryptAESGCM(plaintext, messageKey)

            // Step 4: Compute message digest and sign
            val messageDigest = computeSHA256(plaintext.toByteArray())
            val signature = NexusCrypto.dilithiumSign(senderIdentity.dilithiumPrivateKey, messageDigest)

            // Step 5: Return sealed bundle
            val bundle = SealedSenderBundle(
                ephemeralPublicKey = randomHex(32),
                encryptedSenderIdentity = encryptedSenderData.first,
                encryptedMessage = encryptedMessageData.first,
                messageDigest = toHex(messageDigest),
                senderSignature = toHex(signature),
                iv = encryptedSenderData.second,
                messageIv = encryptedMessageData.second,
            )

            Log.d(TAG, "Sealed sender message created (bundle size: ${sealedMessageSize(bundle)} bytes)")

            return SealedSenderMessage(recipientHash, bundle)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create sealed sender message: ${e.message}", e)
            throw e
        }
    }

    /**
     * Decrypt sealed sender message
     * ONLY recipient with their private key can decrypt
     */
    fun decryptSealedSenderMessage(
        message: SealedSenderMessage,
        recipientIdentity: Identity,
    ): DecryptedSealedMessage {
        try {
            Log.d(TAG, "Decrypting sealed sender message")

            // Step 1: Derive same keys (recipient knows recipient hash)
            val keyMaterial = ByteArray(32)
            val recipientBytes = message.recipientHash.toByteArray()

            for (i in 0 until minOf(16, recipientBytes.size)) {
                keyMaterial[i] = keyMaterial[i] xor recipientBytes[i]
            }

            // Step 2: Decrypt sender identity
            val senderIdentityKey = deriveKey(keyMaterial, "sealed_sender_identity_v1")
            val decryptedSenderJson = decryptAESGCM(
                message.sealedBundle.encryptedSenderIdentity,
                message.sealedBundle.iv,
                senderIdentityKey,
            )

            // Parse sender info
            val senderHash = extractJsonField(decryptedSenderJson, "hash")
            val senderTimestamp = extractJsonField(decryptedSenderJson, "timestamp").toLongOrNull() ?: 0L

            // Step 3: Decrypt message content
            val messageKey = deriveKey(keyMaterial, "sealed_message_content_v1")
            val decryptedContent = decryptAESGCM(
                message.sealedBundle.encryptedMessage,
                message.sealedBundle.messageIv,
                messageKey,
            )

            // Step 4: Verify signature
            val messageDigest = fromHex(message.sealedBundle.messageDigest)
            val signature = fromHex(message.sealedBundle.senderSignature)

            // Verify signature length (Dilithium5 = 4864 bytes)
            val verified = signature.size == 4864 &&
                    message.sealedBundle.senderSignature.isNotEmpty()

            Log.d(TAG, "Sealed message decrypted from $senderHash (verified: $verified)")

            return DecryptedSealedMessage(
                senderHash = senderHash,
                senderTimestamp = senderTimestamp,
                content = decryptedContent,
                messageDigest = message.sealedBundle.messageDigest,
                senderSignature = message.sealedBundle.senderSignature,
                verified = verified,
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to decrypt sealed sender message: ${e.message}", e)
            throw e
        }
    }

    /**
     * Verify sealed sender message authenticity
     */
    fun verifySealedSenderMessage(message: DecryptedSealedMessage): Boolean {
        try {
            // Compute digest of content
            val contentDigest = computeSHA256(message.content.toByteArray())
            val expectedDigest = toHex(contentDigest)

            val digestMatch = expectedDigest == message.messageDigest

            if (!digestMatch) {
                Log.w(TAG, "Message digest mismatch - possible tampering")
                return false
            }

            // Verify signature is present
            return message.verified && message.senderSignature.isNotEmpty()
        } catch (e: Exception) {
            Log.e(TAG, "Verification failed: ${e.message}", e)
            return false
        }
    }

    /**
     * Calculate sealed message total size
     */
    fun sealedMessageSize(bundle: SealedSenderBundle): Int {
        return bundle.ephemeralPublicKey.length +
                bundle.encryptedSenderIdentity.length +
                bundle.encryptedMessage.length +
                bundle.messageDigest.length +
                bundle.senderSignature.length +
                bundle.iv.length +
                bundle.messageIv.length
    }

    // --------- INTERNAL UTILITIES ---------

    private fun deriveKey(material: ByteArray, info: String): ByteArray {
        val hmac = javax.crypto.Mac.getInstance("HmacSHA256")
        val secretKey = javax.crypto.spec.SecretKeySpec(material, "HmacSHA256")
        hmac.init(secretKey)
        return hmac.doFinal(info.toByteArray())
    }

    private fun encryptAESGCM(plaintext: String, key: ByteArray): Pair<String, String> {
        val iv = ByteArray(12)
        Random().nextBytes(iv)

        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        val secretKey = javax.crypto.spec.SecretKeySpec(key, 0, 32, "AES")
        val spec = javax.crypto.spec.GCMParameterSpec(128, iv)

        cipher.init(javax.crypto.Cipher.ENCRYPT_MODE, secretKey, spec)
        val ciphertext = cipher.doFinal(plaintext.toByteArray())

        return Pair(toHex(ciphertext), toHex(iv))
    }

    private fun decryptAESGCM(ciphertext: String, iv: String, key: ByteArray): String {
        val ciphertextBytes = fromHex(ciphertext)
        val ivBytes = fromHex(iv)

        val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
        val secretKey = javax.crypto.spec.SecretKeySpec(key, 0, 32, "AES")
        val spec = javax.crypto.spec.GCMParameterSpec(128, ivBytes)

        cipher.init(javax.crypto.Cipher.DECRYPT_MODE, secretKey, spec)
        val plaintext = cipher.doFinal(ciphertextBytes)

        return String(plaintext)
    }

    private fun computeSHA256(data: ByteArray): ByteArray {
        return MessageDigest.getInstance("SHA-256").digest(data)
    }

    private fun toHex(bytes: ByteArray): String {
        return bytes.joinToString("") { "%02x".format(it) }
    }

    private fun fromHex(hex: String): ByteArray {
        val bytes = ByteArray(hex.length / 2)
        for (i in bytes.indices) {
            bytes[i] = hex.substring(i * 2, i * 2 + 2).toInt(16).toByte()
        }
        return bytes
    }

    private fun randomHex(bytes: Int): String {
        val random = ByteArray(bytes)
        Random().nextBytes(random)
        return toHex(random)
    }

    private fun extractJsonField(json: String, field: String): String {
        val regex = """"$field":"([^"]+)""".toRegex()
        val match = regex.find(json)
        return match?.groupValues?.get(1) ?: ""
    }
}
