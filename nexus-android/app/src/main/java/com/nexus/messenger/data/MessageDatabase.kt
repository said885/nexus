package com.nexus.messenger.data

import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteDatabase
import android.util.Base64
import net.sqlcipher.database.SQLiteDatabase as CipherDB
import net.sqlcipher.database.SQLiteOpenHelper

private const val DB_NAME = "nexus_messages.db"
private const val DB_VERSION = 1

// -------------------------------------------------------------------------
// Entity data classes
// -------------------------------------------------------------------------

data class ConversationEntity(
    val id: Long = 0,
    val participantHash: String,
    val participantName: String?,
    val lastMessagePreview: String?,    // Stored encrypted; decrypted for display
    val lastMessageTime: Long,
    val unreadCount: Int,
    val verified: Boolean = false,
    val ratchetStateSerialized: String?,  // Base64-encoded serialized ratchet state
)

data class MessageEntity(
    val id: Long = 0,
    val conversationId: Long,
    val senderHash: String,
    val content: String,             // Encrypted content (Base64)
    val contentIv: String,           // IV for content decryption (Base64)
    val timestamp: Long,
    val delivered: Boolean = false,
    val read: Boolean = false,
    val autoDeleteAt: Long?,         // Epoch millis, null = never
    val isMine: Boolean,
)

data class PreKeyEntity(
    val id: Int,
    val publicKey: String,           // Base64
    val privateKey: String,          // Base64 (stored encrypted)
    val used: Boolean = false,
    val createdAt: Long,
)

// -------------------------------------------------------------------------
// Database helper (SQLCipher)
// -------------------------------------------------------------------------

class NexusDatabase(
    context: Context,
    private val dbKey: String,
) : SQLiteOpenHelper(
    context,
    DB_NAME,
    null,
    DB_VERSION
) {

    init {
        CipherDB.loadLibs(context)
    }

    private fun getWritable(): CipherDB =
        super.getWritableDatabase(dbKey) as CipherDB

    private fun getReadable(): CipherDB =
        super.getReadableDatabase(dbKey) as CipherDB

    // Suppress the parent method to force SQLCipher usage
    override fun getWritableDatabase(): SQLiteDatabase =
        super.getWritableDatabase(dbKey)

    override fun getReadableDatabase(): SQLiteDatabase =
        super.getReadableDatabase(dbKey)

    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL("""
            CREATE TABLE conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                participant_hash TEXT NOT NULL UNIQUE,
                participant_name TEXT,
                last_message_preview TEXT,
                last_message_time INTEGER NOT NULL DEFAULT 0,
                unread_count INTEGER NOT NULL DEFAULT 0,
                verified INTEGER NOT NULL DEFAULT 0,
                ratchet_state TEXT
            )
        """.trimIndent())

        db.execSQL("""
            CREATE TABLE messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id INTEGER NOT NULL,
                sender_hash TEXT NOT NULL,
                content TEXT NOT NULL,
                content_iv TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                delivered INTEGER NOT NULL DEFAULT 0,
                read_status INTEGER NOT NULL DEFAULT 0,
                auto_delete_at INTEGER,
                is_mine INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )
        """.trimIndent())

        db.execSQL("""
            CREATE TABLE prekeys (
                id INTEGER PRIMARY KEY,
                public_key TEXT NOT NULL,
                private_key TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )
        """.trimIndent())

        db.execSQL("CREATE INDEX idx_messages_conv ON messages(conversation_id)")
        db.execSQL("CREATE INDEX idx_messages_timestamp ON messages(timestamp)")
        db.execSQL("CREATE INDEX idx_prekeys_used ON prekeys(used)")
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        // Future migrations here
    }

    override fun onConfigure(db: SQLiteDatabase) {
        db.execSQL("PRAGMA foreign_keys = ON")
        db.execSQL("PRAGMA journal_mode = WAL")
    }

    // -------------------------------------------------------------------------
    // Conversations DAO
    // -------------------------------------------------------------------------

    fun insertOrUpdateConversation(conv: ConversationEntity): Long {
        val db = getWritable()
        val values = ContentValues().apply {
            put("participant_hash", conv.participantHash)
            put("participant_name", conv.participantName)
            put("last_message_preview", conv.lastMessagePreview)
            put("last_message_time", conv.lastMessageTime)
            put("unread_count", conv.unreadCount)
            put("verified", if (conv.verified) 1 else 0)
            put("ratchet_state", conv.ratchetStateSerialized)
        }
        val existing = getConversationByHash(conv.participantHash)
        return if (existing != null) {
            db.update("conversations", values, "participant_hash = ?", arrayOf(conv.participantHash))
            existing.id
        } else {
            db.insert("conversations", null, values)
        }
    }

    fun getAllConversations(): List<ConversationEntity> {
        val db = getReadable()
        val cursor = db.query(
            "conversations", null, null, null, null, null,
            "last_message_time DESC"
        )
        return cursor.use { it.readConversations() }
    }

    fun getConversationByHash(participantHash: String): ConversationEntity? {
        val db = getReadable()
        val cursor = db.query(
            "conversations", null,
            "participant_hash = ?", arrayOf(participantHash),
            null, null, null, "1"
        )
        return cursor.use { it.readConversations().firstOrNull() }
    }

    fun deleteConversation(id: Long) {
        val db = getWritable()
        db.delete("conversations", "id = ?", arrayOf(id.toString()))
    }

    fun markConversationVerified(participantHash: String, verified: Boolean) {
        val db = getWritable()
        val values = ContentValues().apply {
            put("verified", if (verified) 1 else 0)
        }
        db.update("conversations", values, "participant_hash = ?", arrayOf(participantHash))
    }

    fun updateRatchetState(participantHash: String, ratchetState: String) {
        val db = getWritable()
        val values = ContentValues().apply {
            put("ratchet_state", ratchetState)
        }
        db.update("conversations", values, "participant_hash = ?", arrayOf(participantHash))
    }

    fun clearUnreadCount(participantHash: String) {
        val db = getWritable()
        val values = ContentValues().apply { put("unread_count", 0) }
        db.update("conversations", values, "participant_hash = ?", arrayOf(participantHash))
    }

    // -------------------------------------------------------------------------
    // Messages DAO
    // -------------------------------------------------------------------------

    fun insertMessage(msg: MessageEntity): Long {
        val db = getWritable()
        val values = ContentValues().apply {
            put("conversation_id", msg.conversationId)
            put("sender_hash", msg.senderHash)
            put("content", msg.content)
            put("content_iv", msg.contentIv)
            put("timestamp", msg.timestamp)
            put("delivered", if (msg.delivered) 1 else 0)
            put("read_status", if (msg.read) 1 else 0)
            put("auto_delete_at", msg.autoDeleteAt)
            put("is_mine", if (msg.isMine) 1 else 0)
        }
        return db.insert("messages", null, values)
    }

    fun getMessagesForConversation(
        conversationId: Long,
        limit: Int = 50,
        offset: Int = 0,
    ): List<MessageEntity> {
        val db = getReadable()
        val cursor = db.query(
            "messages", null,
            "conversation_id = ?", arrayOf(conversationId.toString()),
            null, null,
            "timestamp ASC",
            "$offset,$limit"
        )
        return cursor.use { it.readMessages() }
    }

    fun markMessageDelivered(messageId: Long) {
        val db = getWritable()
        val values = ContentValues().apply { put("delivered", 1) }
        db.update("messages", values, "id = ?", arrayOf(messageId.toString()))
    }

    fun markMessageRead(messageId: Long) {
        val db = getWritable()
        val values = ContentValues().apply { put("read_status", 1) }
        db.update("messages", values, "id = ?", arrayOf(messageId.toString()))
    }

    fun deleteExpiredMessages(): Int {
        val db = getWritable()
        val now = System.currentTimeMillis()
        return db.delete("messages", "auto_delete_at IS NOT NULL AND auto_delete_at <= ?", arrayOf(now.toString()))
    }

    fun deleteMessage(messageId: Long) {
        val db = getWritable()
        db.delete("messages", "id = ?", arrayOf(messageId.toString()))
    }

    fun setMessageAutoDelete(messageId: Long, deleteAt: Long?) {
        val db = getWritable()
        val values = ContentValues().apply { put("auto_delete_at", deleteAt) }
        db.update("messages", values, "id = ?", arrayOf(messageId.toString()))
    }

    // -------------------------------------------------------------------------
    // PreKeys DAO
    // -------------------------------------------------------------------------

    fun insertPreKey(key: PreKeyEntity) {
        val db = getWritable()
        val values = ContentValues().apply {
            put("id", key.id)
            put("public_key", key.publicKey)
            put("private_key", key.privateKey)
            put("used", if (key.used) 1 else 0)
            put("created_at", key.createdAt)
        }
        db.insertWithOnConflict("prekeys", null, values, CipherDB.CONFLICT_REPLACE)
    }

    fun getUnusedPreKey(): PreKeyEntity? {
        val db = getReadable()
        val cursor = db.query(
            "prekeys", null,
            "used = 0", null,
            null, null, "created_at ASC", "1"
        )
        return cursor.use { it.readPreKeys().firstOrNull() }
    }

    fun markPreKeyUsed(id: Int) {
        val db = getWritable()
        val values = ContentValues().apply { put("used", 1) }
        db.update("prekeys", values, "id = ?", arrayOf(id.toString()))
    }

    fun getPreKeyCount(): Int {
        val db = getReadable()
        val cursor = db.rawQuery("SELECT COUNT(*) FROM prekeys WHERE used = 0", null)
        return cursor.use { if (it.moveToFirst()) it.getInt(0) else 0 }
    }

    // -------------------------------------------------------------------------
    // Cursor extensions
    // -------------------------------------------------------------------------

    private fun Cursor.readConversations(): List<ConversationEntity> {
        val list = mutableListOf<ConversationEntity>()
        while (moveToNext()) {
            list.add(
                ConversationEntity(
                    id = getLong(getColumnIndexOrThrow("id")),
                    participantHash = getString(getColumnIndexOrThrow("participant_hash")),
                    participantName = getStringOrNull(getColumnIndexOrThrow("participant_name")),
                    lastMessagePreview = getStringOrNull(getColumnIndexOrThrow("last_message_preview")),
                    lastMessageTime = getLong(getColumnIndexOrThrow("last_message_time")),
                    unreadCount = getInt(getColumnIndexOrThrow("unread_count")),
                    verified = getInt(getColumnIndexOrThrow("verified")) == 1,
                    ratchetStateSerialized = getStringOrNull(getColumnIndexOrThrow("ratchet_state")),
                )
            )
        }
        return list
    }

    private fun Cursor.readMessages(): List<MessageEntity> {
        val list = mutableListOf<MessageEntity>()
        while (moveToNext()) {
            val autoDeleteCol = getColumnIndexOrThrow("auto_delete_at")
            list.add(
                MessageEntity(
                    id = getLong(getColumnIndexOrThrow("id")),
                    conversationId = getLong(getColumnIndexOrThrow("conversation_id")),
                    senderHash = getString(getColumnIndexOrThrow("sender_hash")),
                    content = getString(getColumnIndexOrThrow("content")),
                    contentIv = getString(getColumnIndexOrThrow("content_iv")),
                    timestamp = getLong(getColumnIndexOrThrow("timestamp")),
                    delivered = getInt(getColumnIndexOrThrow("delivered")) == 1,
                    read = getInt(getColumnIndexOrThrow("read_status")) == 1,
                    autoDeleteAt = if (isNull(autoDeleteCol)) null else getLong(autoDeleteCol),
                    isMine = getInt(getColumnIndexOrThrow("is_mine")) == 1,
                )
            )
        }
        return list
    }

    private fun Cursor.readPreKeys(): List<PreKeyEntity> {
        val list = mutableListOf<PreKeyEntity>()
        while (moveToNext()) {
            list.add(
                PreKeyEntity(
                    id = getInt(getColumnIndexOrThrow("id")),
                    publicKey = getString(getColumnIndexOrThrow("public_key")),
                    privateKey = getString(getColumnIndexOrThrow("private_key")),
                    used = getInt(getColumnIndexOrThrow("used")) == 1,
                    createdAt = getLong(getColumnIndexOrThrow("created_at")),
                )
            )
        }
        return list
    }

    private fun Cursor.getStringOrNull(columnIndex: Int): String? =
        if (isNull(columnIndex)) null else getString(columnIndex)
}
