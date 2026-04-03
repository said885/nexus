package com.nexus.messenger.crypto

import org.bouncycastle.crypto.agreement.X25519Agreement
import org.bouncycastle.crypto.digests.SHA3Digest
import org.bouncycastle.crypto.generators.HKDFBytesGenerator
import org.bouncycastle.crypto.generators.X25519KeyPairGenerator
import org.bouncycastle.crypto.params.HKDFParameters
import org.bouncycastle.crypto.params.KeyParameter
import org.bouncycastle.crypto.params.X25519KeyGenerationParameters
import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import org.bouncycastle.crypto.params.X25519PublicKeyParameters
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumKeyGenerationParameters
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumKeyPairGenerator
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumParameters
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumPrivateKeyParameters
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumPublicKeyParameters
import org.bouncycastle.pqc.crypto.crystals.dilithium.DilithiumSigner
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberKEMExtractor
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberKEMGenerator
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberKeyGenerationParameters
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberKeyPairGenerator
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberParameters
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberPrivateKeyParameters
import org.bouncycastle.pqc.crypto.crystals.kyber.KyberPublicKeyParameters
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import java.security.SecureRandom

/** Full identity: PQ signing (Dilithium5) + PQ KEM (Kyber1024) + classical DH (X25519) */
data class NexusIdentity(
    val dilithiumPublicKey: ByteArray,
    val dilithiumPrivateKey: ByteArray,
    val kyberPublicKey: ByteArray,
    val kyberPrivateKey: ByteArray,
    val x25519PublicKey: ByteArray,
    val x25519PrivateKey: ByteArray,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is NexusIdentity) return false
        return dilithiumPublicKey.contentEquals(other.dilithiumPublicKey)
    }
    override fun hashCode(): Int = dilithiumPublicKey.contentHashCode()
}

/** Prekey bundle published to relay for X3DH */
data class PreKeyBundle(
    val identityKey: ByteArray,         // Kyber1024 public key of recipient
    val signedPreKey: ByteArray,        // X25519 ephemeral pre-key
    val signedPreKeySignature: ByteArray, // Dilithium5 signature over signedPreKey
    val oneTimePreKey: ByteArray?,      // Optional one-time X25519 key
    val oneTimePreKeyId: Int?,
)

/** Double-ratchet state */
data class RatchetState(
    var rootKey: ByteArray,
    var sendChainKey: ByteArray?,
    var recvChainKey: ByteArray?,
    var sendMsgN: Int = 0,
    var recvMsgN: Int = 0,
    var prevSendCount: Int = 0,
    var dhSendPriv: ByteArray,
    var dhSendPub: ByteArray,
    var dhRemotePub: ByteArray?,
    val skippedKeys: MutableMap<Pair<String, Int>, ByteArray> = mutableMapOf(),
)

data class EncryptedMessage(
    val header: MessageHeader,
    val ciphertext: ByteArray,
)

data class MessageHeader(
    val dhPublic: ByteArray,
    val msgN: Int,
    val prevChainLen: Int,
)

object NexusCrypto {

    private val random = SecureRandom.getInstanceStrong()

    // -------------------------------------------------------------------------
    // Identity generation
    // -------------------------------------------------------------------------

    fun generateIdentity(): NexusIdentity {
        // Dilithium5 signing keypair
        val dilithiumGen = DilithiumKeyPairGenerator()
        dilithiumGen.init(DilithiumKeyGenerationParameters(random, DilithiumParameters.dilithium5))
        val dilithiumKP = dilithiumGen.generateKeyPair()
        val dilPub = (dilithiumKP.public as DilithiumPublicKeyParameters).encoded
        val dilPriv = (dilithiumKP.private as DilithiumPrivateKeyParameters).encoded

        // Kyber1024 KEM keypair
        val kyberGen = KyberKeyPairGenerator()
        kyberGen.init(KyberKeyGenerationParameters(random, KyberParameters.kyber1024))
        val kyberKP = kyberGen.generateKeyPair()
        val kyberPub = (kyberKP.public as KyberPublicKeyParameters).encoded
        val kyberPriv = (kyberKP.private as KyberPrivateKeyParameters).encoded

        // X25519 DH keypair
        val x25519Gen = X25519KeyPairGenerator()
        x25519Gen.init(X25519KeyGenerationParameters(random))
        val x25519KP = x25519Gen.generateKeyPair()
        val x25519Pub = (x25519KP.public as X25519PublicKeyParameters).encoded
        val x25519Priv = (x25519KP.private as X25519PrivateKeyParameters).encoded

        return NexusIdentity(dilPub, dilPriv, kyberPub, kyberPriv, x25519Pub, x25519Priv)
    }

    // -------------------------------------------------------------------------
    // PQ-X3DH (hybrid Kyber1024 + X25519)
    // -------------------------------------------------------------------------

    /**
     * X3DH sender side.
     * @return Pair(masterSecret, ephemeralPublicKeyBytes)
     *
     * Protocol:
     *   1. Alice generates an ephemeral X25519 key pair.
     *   2. Alice encapsulates to Bob's Kyber public key  -> (ss_pq, ct_kyber)
     *   3. Alice performs X25519(aliceEph, bobSignedPreKey) -> dh1
     *   4. Alice performs X25519(aliceEph, bobOTPK?) -> dh2 (optional)
     *   5. masterSecret = HKDF(ss_pq || dh1 || dh2, ...)
     */
    fun x3dhSend(myIdentity: NexusIdentity, bobBundle: PreKeyBundle): Pair<ByteArray, ByteArray> {
        // 1. Generate ephemeral X25519 keypair
        val ephGen = X25519KeyPairGenerator()
        ephGen.init(X25519KeyGenerationParameters(random))
        val ephKP = ephGen.generateKeyPair()
        val ephPub = (ephKP.public as X25519PublicKeyParameters).encoded
        val ephPriv = (ephKP.private as X25519PrivateKeyParameters)

        // 2. Kyber encapsulation to Bob's identity Kyber key
        val bobKyberPub = KyberPublicKeyParameters(KyberParameters.kyber1024, bobBundle.identityKey)
        val kemGen = KyberKEMGenerator(random)
        val encapsulatedSecret = kemGen.generateEncapsulated(bobKyberPub)
        val ssPq = encapsulatedSecret.secret
        val ctKyber = encapsulatedSecret.encapsulation

        // 3. X25519(aliceEph, bobSignedPreKey)
        val bobSpkPub = X25519PublicKeyParameters(bobBundle.signedPreKey, 0)
        val x25519Agreement1 = X25519Agreement()
        x25519Agreement1.init(ephPriv)
        val dh1 = ByteArray(x25519Agreement1.agreementSize)
        x25519Agreement1.calculateAgreement(bobSpkPub, dh1, 0)

        // 4. Optional X25519 with one-time pre-key
        val dh2 = if (bobBundle.oneTimePreKey != null) {
            val bobOTPK = X25519PublicKeyParameters(bobBundle.oneTimePreKey, 0)
            val x25519Agreement2 = X25519Agreement()
            x25519Agreement2.init(ephPriv)
            ByteArray(x25519Agreement2.agreementSize).also {
                x25519Agreement2.calculateAgreement(bobOTPK, it, 0)
            }
        } else ByteArray(32) // Zero-pad to 32 bytes when no OTPK (CRITICAL: must match receiver!)

        // 5. Combine: ikm = ssPq || ctKyber || dh1 || dh2
        val ikm = ssPq + ctKyber + dh1 + dh2
        val salt = "NEXUS-X3DH-v1".toByteArray(Charsets.UTF_8)
        val info = "nexus-master-secret".toByteArray(Charsets.UTF_8)
        val masterSecret = hkdf(ikm, salt, info, 64)

        // Wipe sensitive material
        wipe(ssPq, dh1, dh2, ikm)

        // Ephemeral public = ephPub || ctKyber (so receiver can decapsulate)
        val combinedEph = ephPub + ctKyber
        return Pair(masterSecret, combinedEph)
    }

    /**
     * X3DH receiver side.
     * aliceEphKey = aliceEphX25519Pub (32 bytes) || kyberCiphertext
     */
    fun x3dhReceive(
        myIdentity: NexusIdentity,
        aliceEphKey: ByteArray,
        aliceIdentityKey: ByteArray,
    ): ByteArray {
        // Split aliceEphKey: first 32 bytes = X25519 pub, rest = Kyber ciphertext
        val aliceX25519Pub = aliceEphKey.copyOfRange(0, 32)
        val kyberCt = aliceEphKey.copyOfRange(32, aliceEphKey.size)

        // Kyber decapsulation
        val myKyberPriv = KyberPrivateKeyParameters(KyberParameters.kyber1024, myIdentity.kyberPrivateKey)
        val kemExtractor = KyberKEMExtractor(myKyberPriv)
        val ssPq = kemExtractor.extractSecret(kyberCt)

        // X25519: Bob's signed pre-key (we reuse x25519 identity key as SPK for simplicity)
        val myX25519Priv = X25519PrivateKeyParameters(myIdentity.x25519PrivateKey, 0)
        val aliceX25519PubParams = X25519PublicKeyParameters(aliceX25519Pub, 0)
        val x25519Agreement = X25519Agreement()
        x25519Agreement.init(myX25519Priv)
        val dh1 = ByteArray(x25519Agreement.agreementSize)
        x25519Agreement.calculateAgreement(aliceX25519PubParams, dh1, 0)

        // CRITICAL: Must include zero-padded dh2 even when no OTPK, to match sender's IKM!
        // This was a bug: receiver was omitting dh2, causing master secret mismatch
        val dh2 = ByteArray(32) // Zero-pad when no OTPK (matches sender side)

        val ikm = ssPq + kyberCt + dh1 + dh2
        val salt = "NEXUS-X3DH-v1".toByteArray(Charsets.UTF_8)
        val info = "nexus-master-secret".toByteArray(Charsets.UTF_8)
        val masterSecret = hkdf(ikm, salt, info, 64)

        wipe(ssPq, dh1, dh2, ikm)
        return masterSecret
    }

    // -------------------------------------------------------------------------
    // Double Ratchet
    // -------------------------------------------------------------------------

    fun initRatchetSender(sharedSecret: ByteArray, remoteRatchetKey: ByteArray): RatchetState {
        // Generate our initial DH ratchet keypair
        val dhGen = X25519KeyPairGenerator()
        dhGen.init(X25519KeyGenerationParameters(random))
        val dhKP = dhGen.generateKeyPair()
        val dhPub = (dhKP.public as X25519PublicKeyParameters).encoded
        val dhPriv = (dhKP.private as X25519PrivateKeyParameters).encoded

        // Perform initial DH ratchet step
        val dhPrivParams = X25519PrivateKeyParameters(dhPriv, 0)
        val remotePubParams = X25519PublicKeyParameters(remoteRatchetKey, 0)
        val agreement = X25519Agreement()
        agreement.init(dhPrivParams)
        val dhOut = ByteArray(agreement.agreementSize)
        agreement.calculateAgreement(remotePubParams, dhOut, 0)

        // KDF ratchet: (rootKey, chainKey) = HKDF(rootKey, dhOut)
        val kdfOut = kdfRootKey(sharedSecret, dhOut)
        val rootKey = kdfOut.copyOfRange(0, 32)
        val sendChainKey = kdfOut.copyOfRange(32, 64)

        wipe(dhOut, kdfOut)

        return RatchetState(
            rootKey = rootKey,
            sendChainKey = sendChainKey,
            recvChainKey = null,
            dhSendPriv = dhPriv,
            dhSendPub = dhPub,
            dhRemotePub = remoteRatchetKey,
        )
    }

    fun initRatchetReceiver(
        sharedSecret: ByteArray,
        myRatchetKeyPair: Pair<ByteArray, ByteArray>,
    ): RatchetState {
        // myRatchetKeyPair = (privateKey, publicKey)
        return RatchetState(
            rootKey = sharedSecret.copyOfRange(0, 32),
            sendChainKey = null,
            recvChainKey = sharedSecret.copyOfRange(32, 64),
            dhSendPriv = myRatchetKeyPair.first,
            dhSendPub = myRatchetKeyPair.second,
            dhRemotePub = null,
        )
    }

    fun ratchetEncrypt(
        state: RatchetState,
        plaintext: ByteArray,
        assocData: ByteArray,
    ): EncryptedMessage {
        // Get message key from send chain
        val (msgKey, nextChainKey) = kdfChainKey(state.sendChainKey ?: error("No send chain key"))
        state.sendChainKey = nextChainKey

        val header = MessageHeader(
            dhPublic = state.dhSendPub.copyOf(),
            msgN = state.sendMsgN,
            prevChainLen = state.prevSendCount,
        )
        state.sendMsgN++

        val headerBytes = encodeHeader(header)
        val aad = assocData + headerBytes
        val nonce = randomBytes(12)
        val ciphertext = chachaEncrypt(msgKey, nonce, plaintext, aad)

        wipe(msgKey, nextChainKey)

        return EncryptedMessage(header, nonce + ciphertext)
    }

    fun ratchetDecrypt(
        state: RatchetState,
        msg: EncryptedMessage,
        assocData: ByteArray,
    ): ByteArray {
        // Check skipped message keys first
        val skippedKey = state.skippedKeys[Pair(msg.header.dhPublic.toHex(), msg.header.msgN)]
        if (skippedKey != null) {
            state.skippedKeys.remove(Pair(msg.header.dhPublic.toHex(), msg.header.msgN))
            return decryptWithMsgKey(skippedKey, msg, assocData)
        }

        // Check if we need a DH ratchet step
        val remotePubHex = msg.header.dhPublic.toHex()
        val currentRemotePubHex = state.dhRemotePub?.toHex()

        if (remotePubHex != currentRemotePubHex) {
            // Skip remaining messages in current recv chain
            if (state.recvChainKey != null) {
                skipMessageKeys(state, msg.header.prevChainLen)
            }
            // DH ratchet step
            dhRatchetStep(state, msg.header.dhPublic)
        }

        // Skip forward to the correct message
        skipMessageKeys(state, msg.header.msgN)

        val (msgKey, nextChainKey) = kdfChainKey(state.recvChainKey ?: error("No recv chain key"))
        state.recvChainKey = nextChainKey
        state.recvMsgN++

        val plaintext = decryptWithMsgKey(msgKey, msg, assocData)
        wipe(msgKey, nextChainKey)
        return plaintext
    }

    private fun decryptWithMsgKey(
        msgKey: ByteArray,
        msg: EncryptedMessage,
        assocData: ByteArray,
    ): ByteArray {
        val headerBytes = encodeHeader(msg.header)
        val aad = assocData + headerBytes
        val nonce = msg.ciphertext.copyOfRange(0, 12)
        val ct = msg.ciphertext.copyOfRange(12, msg.ciphertext.size)
        return chachaDecrypt(msgKey, nonce, ct, aad)
    }

    private fun dhRatchetStep(state: RatchetState, remotePub: ByteArray) {
        state.prevSendCount = state.sendMsgN
        state.sendMsgN = 0
        state.recvMsgN = 0
        state.dhRemotePub = remotePub

        // Receive ratchet: derive recv chain key
        val recvDhOut = x25519DH(state.dhSendPriv, remotePub)
        val recvKdf = kdfRootKey(state.rootKey, recvDhOut)
        state.rootKey = recvKdf.copyOfRange(0, 32)
        state.recvChainKey = recvKdf.copyOfRange(32, 64)
        wipe(recvDhOut, recvKdf)

        // Generate new DH keypair for send ratchet
        val dhGen = X25519KeyPairGenerator()
        dhGen.init(X25519KeyGenerationParameters(random))
        val newKP = dhGen.generateKeyPair()
        state.dhSendPub = (newKP.public as X25519PublicKeyParameters).encoded
        state.dhSendPriv = (newKP.private as X25519PrivateKeyParameters).encoded

        // Send ratchet
        val sendDhOut = x25519DH(state.dhSendPriv, remotePub)
        val sendKdf = kdfRootKey(state.rootKey, sendDhOut)
        state.rootKey = sendKdf.copyOfRange(0, 32)
        state.sendChainKey = sendKdf.copyOfRange(32, 64)
        wipe(sendDhOut, sendKdf)
    }

    private fun skipMessageKeys(state: RatchetState, until: Int) {
        val maxSkip = 100
        val chainKey = state.recvChainKey ?: return

        // Check total skip count BEFORE processing to prevent DoS
        val skipCount = until - state.recvMsgN
        if (skipCount > maxSkip) {
            throw IllegalStateException("Too many skipped messages: $skipCount > $maxSkip (DoS protection)")
        }

        var ck = chainKey.copyOf()
        while (state.recvMsgN < until) {
            val (msgKey, nextCk) = kdfChainKey(ck)
            state.skippedKeys[Pair(state.dhRemotePub?.toHex() ?: "", state.recvMsgN)] = msgKey
            ck = nextCk
            state.recvMsgN++
        }
        state.recvChainKey = ck
    }

    private fun x25519DH(privKeyBytes: ByteArray, pubKeyBytes: ByteArray): ByteArray {
        val priv = X25519PrivateKeyParameters(privKeyBytes, 0)
        val pub = X25519PublicKeyParameters(pubKeyBytes, 0)
        val agreement = X25519Agreement()
        agreement.init(priv)
        val out = ByteArray(agreement.agreementSize)
        agreement.calculateAgreement(pub, out, 0)
        return out
    }

    /** KDF for root key step: returns 64 bytes (32 new root + 32 chain) */
    private fun kdfRootKey(rootKey: ByteArray, dhOutput: ByteArray): ByteArray {
        val info = "nexus-ratchet-root".toByteArray(Charsets.UTF_8)
        return hkdf(dhOutput, rootKey, info, 64)
    }

    /** KDF for chain key: returns (messageKey[32], nextChainKey[32]) */
    private fun kdfChainKey(chainKey: ByteArray): Pair<ByteArray, ByteArray> {
        val msgKeyInfo = "nexus-msg-key".toByteArray(Charsets.UTF_8)
        val chainKeyInfo = "nexus-chain-key".toByteArray(Charsets.UTF_8)
        val msgKey = hkdf(chainKey, null, msgKeyInfo, 32)
        val nextChainKey = hkdf(chainKey, null, chainKeyInfo, 32)
        return Pair(msgKey, nextChainKey)
    }

    private fun encodeHeader(header: MessageHeader): ByteArray {
        val buf = ByteArray(header.dhPublic.size + 8)
        header.dhPublic.copyInto(buf, 0)
        buf[header.dhPublic.size] = (header.msgN shr 24).toByte()
        buf[header.dhPublic.size + 1] = (header.msgN shr 16).toByte()
        buf[header.dhPublic.size + 2] = (header.msgN shr 8).toByte()
        buf[header.dhPublic.size + 3] = header.msgN.toByte()
        buf[header.dhPublic.size + 4] = (header.prevChainLen shr 24).toByte()
        buf[header.dhPublic.size + 5] = (header.prevChainLen shr 16).toByte()
        buf[header.dhPublic.size + 6] = (header.prevChainLen shr 8).toByte()
        buf[header.dhPublic.size + 7] = header.prevChainLen.toByte()
        return buf
    }

    // -------------------------------------------------------------------------
    // HKDF (SHA3-512)
    // -------------------------------------------------------------------------

    fun hkdf(ikm: ByteArray, salt: ByteArray?, info: ByteArray, length: Int): ByteArray {
        val generator = HKDFBytesGenerator(SHA3Digest(512))
        val params = if (salt != null && salt.isNotEmpty()) {
            HKDFParameters(ikm, salt, info)
        } else {
            HKDFParameters.skipExtractParameters(ikm, info)
        }
        generator.init(params)
        val out = ByteArray(length)
        generator.generateBytes(out, 0, length)
        return out
    }

    // -------------------------------------------------------------------------
    // ChaCha20-Poly1305
    // -------------------------------------------------------------------------

    fun chachaEncrypt(key: ByteArray, nonce: ByteArray, plaintext: ByteArray, aad: ByteArray): ByteArray {
        val cipher = ChaCha20Poly1305()
        val params = AEADParameters(KeyParameter(key), 128, nonce, aad)
        cipher.init(true, params)
        val out = ByteArray(cipher.getOutputSize(plaintext.size))
        val len = cipher.processBytes(plaintext, 0, plaintext.size, out, 0)
        cipher.doFinal(out, len)
        return out
    }

    fun chachaDecrypt(key: ByteArray, nonce: ByteArray, ciphertext: ByteArray, aad: ByteArray): ByteArray {
        val cipher = ChaCha20Poly1305()
        val params = AEADParameters(KeyParameter(key), 128, nonce, aad)
        cipher.init(false, params)
        val out = ByteArray(cipher.getOutputSize(ciphertext.size))
        val len = cipher.processBytes(ciphertext, 0, ciphertext.size, out, 0)
        cipher.doFinal(out, len)
        return out
    }

    // -------------------------------------------------------------------------
    // Dilithium5 sign/verify
    // -------------------------------------------------------------------------

    fun dilithiumSign(privateKey: ByteArray, message: ByteArray): ByteArray {
        val privParams = DilithiumPrivateKeyParameters(DilithiumParameters.dilithium5, privateKey, null)
        val signer = DilithiumSigner()
        signer.init(true, privParams)
        return signer.generateSignature(message)
    }

    fun dilithiumVerify(publicKey: ByteArray, message: ByteArray, signature: ByteArray): Boolean {
        return try {
            val pubParams = DilithiumPublicKeyParameters(DilithiumParameters.dilithium5, publicKey)
            val verifier = DilithiumSigner()
            verifier.init(false, pubParams)
            verifier.verifySignature(message, signature)
        } catch (e: Exception) {
            false
        }
    }

    // -------------------------------------------------------------------------
    // Kyber1024 KEM
    // -------------------------------------------------------------------------

    fun kyberEncap(publicKey: ByteArray): Pair<ByteArray, ByteArray> {
        val pubParams = KyberPublicKeyParameters(KyberParameters.kyber1024, publicKey)
        val kemGen = KyberKEMGenerator(random)
        val encapsulated = kemGen.generateEncapsulated(pubParams)
        return Pair(encapsulated.secret, encapsulated.encapsulation)
    }

    fun kyberDecap(privateKey: ByteArray, ciphertext: ByteArray): ByteArray {
        val privParams = KyberPrivateKeyParameters(KyberParameters.kyber1024, privateKey)
        val extractor = KyberKEMExtractor(privParams)
        return extractor.extractSecret(ciphertext)
    }

    // -------------------------------------------------------------------------
    // Identity hash (SHA3-256)
    // -------------------------------------------------------------------------

    fun identityHash(publicKey: ByteArray): ByteArray {
        val digest = SHA3Digest(256)
        digest.update(publicKey, 0, publicKey.size)
        val out = ByteArray(32)
        digest.doFinal(out, 0)
        return out
    }

    // -------------------------------------------------------------------------
    // Utilities
    // -------------------------------------------------------------------------

    fun randomBytes(n: Int): ByteArray = ByteArray(n).also { random.nextBytes(it) }

    fun wipe(vararg arrays: ByteArray) {
        arrays.forEach { it.fill(0) }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
