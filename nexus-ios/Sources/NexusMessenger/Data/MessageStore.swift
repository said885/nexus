import CoreData
import CryptoKit
import Foundation

// MARK: - Models

public struct ConversationModel: Identifiable {
    public let id: String
    public let participantHash: String
    public var displayName: String
    public var lastMessage: String
    public var unreadCount: Int
    public var lastUpdated: Date
}

public struct MessageModel: Identifiable {
    public let id: String
    public let conversationId: String
    public var content: String          // Decrypted plaintext (never persisted in clear)
    public let timestamp: Date
    public var status: MessageStatus
    public var autoDeleteAt: Date?
    public var isSent: Bool             // true = sent by us, false = received
}

public enum MessageStatus: String {
    case sending, sent, delivered, read, failed
}

// MARK: - Errors

public enum MessageStoreError: Error, LocalizedError {
    case encryptionFailed
    case decryptionFailed
    case saveFailed(Error)
    case fetchFailed(Error)
    case notFound

    public var errorDescription: String? {
        switch self {
        case .encryptionFailed:    return "Failed to encrypt message"
        case .decryptionFailed:    return "Failed to decrypt stored message"
        case .saveFailed(let e):   return "Save failed: \(e.localizedDescription)"
        case .fetchFailed(let e):  return "Fetch failed: \(e.localizedDescription)"
        case .notFound:            return "Item not found"
        }
    }
}

// MARK: - MessageStore

/// CoreData-backed encrypted message store.
/// All sensitive text fields are AES-GCM encrypted with the store key before
/// being written to SQLite.  The store key itself is protected in the Keychain.
@MainActor
public final class MessageStore: ObservableObject {

    // -------------------------------------------------------------------------
    // MARK: Properties
    // -------------------------------------------------------------------------

    private let container: NSPersistentContainer
    private let storeKey: SymmetricKey

    @Published public private(set) var conversations: [ConversationModel] = []

    // -------------------------------------------------------------------------
    // MARK: Init
    // -------------------------------------------------------------------------

    public init() {
        // Load or generate the store encryption key
        storeKey = Self.loadOrCreateStoreKey()

        // Build the managed object model in code (no .xcdatamodeld file needed)
        let model = Self.buildManagedObjectModel()
        container = NSPersistentContainer(name: "NexusStore", managedObjectModel: model)

        let storeURL = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("NexusStore.sqlite")

        let description = NSPersistentStoreDescription(url: storeURL)
        description.type = NSSQLiteStoreType
        // Enable WAL mode for better concurrency
        description.setOption(["journal_mode": "WAL"] as NSObject,
                               forKey: NSSQLiteStoreType)
        container.persistentStoreDescriptions = [description]

        // Load is called synchronously in init; in a real app you'd use the async version.
        container.loadPersistentStores { _, error in
            if let error = error {
                // In production: wipe the store and start fresh
                print("[MessageStore] Failed to load store: \(error.localizedDescription)")
            }
        }
        container.viewContext.automaticallyMergesChangesFromParent = true
    }

    // -------------------------------------------------------------------------
    // MARK: Core Data Model (built in code)
    // -------------------------------------------------------------------------

    private static func buildManagedObjectModel() -> NSManagedObjectModel {
        let model = NSManagedObjectModel()

        // Conversation entity
        let conversationEntity = NSEntityDescription()
        conversationEntity.name            = "CDConversation"
        conversationEntity.managedObjectClassName = "NSManagedObject"

        let convIdAttr = NSAttributeDescription()
        convIdAttr.name = "id"; convIdAttr.attributeType = .stringAttributeType; convIdAttr.isOptional = false

        let convParticipantAttr = NSAttributeDescription()
        convParticipantAttr.name = "participantHash"; convParticipantAttr.attributeType = .stringAttributeType; convParticipantAttr.isOptional = false

        let convDisplayNameAttr = NSAttributeDescription()
        convDisplayNameAttr.name = "displayNameEncrypted"; convDisplayNameAttr.attributeType = .binaryDataAttributeType; convDisplayNameAttr.isOptional = true

        let convLastMsgAttr = NSAttributeDescription()
        convLastMsgAttr.name = "lastMessageEncrypted"; convLastMsgAttr.attributeType = .binaryDataAttributeType; convLastMsgAttr.isOptional = true

        let convUnreadAttr = NSAttributeDescription()
        convUnreadAttr.name = "unreadCount"; convUnreadAttr.attributeType = .integer32AttributeType; convUnreadAttr.defaultValue = 0

        let convUpdatedAttr = NSAttributeDescription()
        convUpdatedAttr.name = "lastUpdated"; convUpdatedAttr.attributeType = .dateAttributeType; convUpdatedAttr.isOptional = true

        conversationEntity.properties = [convIdAttr, convParticipantAttr, convDisplayNameAttr, convLastMsgAttr, convUnreadAttr, convUpdatedAttr]

        // Message entity
        let messageEntity = NSEntityDescription()
        messageEntity.name = "CDMessage"
        messageEntity.managedObjectClassName = "NSManagedObject"

        let msgIdAttr = NSAttributeDescription()
        msgIdAttr.name = "id"; msgIdAttr.attributeType = .stringAttributeType; msgIdAttr.isOptional = false

        let msgConvIdAttr = NSAttributeDescription()
        msgConvIdAttr.name = "conversationId"; msgConvIdAttr.attributeType = .stringAttributeType; msgConvIdAttr.isOptional = false

        let msgContentAttr = NSAttributeDescription()
        msgContentAttr.name = "contentEncrypted"; msgContentAttr.attributeType = .binaryDataAttributeType; msgContentAttr.isOptional = false

        let msgTimestampAttr = NSAttributeDescription()
        msgTimestampAttr.name = "timestamp"; msgTimestampAttr.attributeType = .dateAttributeType; msgTimestampAttr.isOptional = false

        let msgStatusAttr = NSAttributeDescription()
        msgStatusAttr.name = "status"; msgStatusAttr.attributeType = .stringAttributeType; msgStatusAttr.defaultValue = "sent"

        let msgAutoDeleteAttr = NSAttributeDescription()
        msgAutoDeleteAttr.name = "autoDeleteAt"; msgAutoDeleteAttr.attributeType = .dateAttributeType; msgAutoDeleteAttr.isOptional = true

        let msgIsSentAttr = NSAttributeDescription()
        msgIsSentAttr.name = "isSent"; msgIsSentAttr.attributeType = .booleanAttributeType; msgIsSentAttr.defaultValue = true

        messageEntity.properties = [msgIdAttr, msgConvIdAttr, msgContentAttr, msgTimestampAttr, msgStatusAttr, msgAutoDeleteAttr, msgIsSentAttr]

        model.entities = [conversationEntity, messageEntity]
        return model
    }

    // -------------------------------------------------------------------------
    // MARK: Store key
    // -------------------------------------------------------------------------

    private static func loadOrCreateStoreKey() -> SymmetricKey {
        let label = "nexus.store.key"
        if let data = try? SecureEnclaveManager.loadKeychainData(label: label) {
            return SymmetricKey(data: data)
        }
        let newKey = SymmetricKey(size: .bits256)
        let keyData = newKey.withUnsafeBytes { Data($0) }
        try? SecureEnclaveManager.saveKeychainData(label: label, data: keyData)
        return newKey
    }

    // -------------------------------------------------------------------------
    // MARK: Encryption helpers
    // -------------------------------------------------------------------------

    private func encrypt(_ string: String) throws -> Data {
        guard let plaintext = string.data(using: .utf8) else { throw MessageStoreError.encryptionFailed }
        let (combined, nonce) = try NexusCrypto.aesgcmEncrypt(key: storeKey, plaintext: plaintext, aad: Data("nexus.store".utf8))
        var blob = nonce
        blob.append(combined)
        return blob
    }

    private func decrypt(_ blob: Data) throws -> String {
        guard blob.count > 12 + 16 else { throw MessageStoreError.decryptionFailed }
        let nonce  = blob.prefix(12)
        let rest   = blob.dropFirst(12)
        let ct     = rest.dropLast(16)
        let tag    = rest.suffix(16)
        let plain  = try NexusCrypto.aesgcmDecrypt(key: storeKey, ciphertext: ct, nonce: nonce, tag: tag, aad: Data("nexus.store".utf8))
        guard let string = String(data: plain, encoding: .utf8) else { throw MessageStoreError.decryptionFailed }
        return string
    }

    // -------------------------------------------------------------------------
    // MARK: Save Message
    // -------------------------------------------------------------------------

    public func saveMessage(
        id: String = UUID().uuidString,
        conversationId: String,
        participantHash: String,
        content: String,
        timestamp: Date = Date(),
        isSent: Bool,
        status: MessageStatus = .sent,
        autoDeleteAfter: TimeInterval? = nil
    ) throws -> MessageModel {
        let ctx = container.viewContext

        // Encrypt the content
        let encryptedContent = try encrypt(content)

        // Upsert message entity
        let msgObj = NSManagedObject(
            entity: container.managedObjectModel.entitiesByName["CDMessage"]!,
            insertInto: ctx
        )
        msgObj.setValue(id,               forKey: "id")
        msgObj.setValue(conversationId,   forKey: "conversationId")
        msgObj.setValue(encryptedContent, forKey: "contentEncrypted")
        msgObj.setValue(timestamp,        forKey: "timestamp")
        msgObj.setValue(status.rawValue,  forKey: "status")
        msgObj.setValue(isSent,           forKey: "isSent")

        let autoDeleteAt: Date?
        if let interval = autoDeleteAfter {
            autoDeleteAt = timestamp.addingTimeInterval(interval)
            msgObj.setValue(autoDeleteAt, forKey: "autoDeleteAt")
        } else {
            autoDeleteAt = nil
        }

        // Upsert conversation
        try upsertConversation(
            id: conversationId,
            participantHash: participantHash,
            lastMessage: content,
            timestamp: timestamp,
            in: ctx
        )

        do {
            try ctx.save()
        } catch {
            throw MessageStoreError.saveFailed(error)
        }

        return MessageModel(
            id: id,
            conversationId: conversationId,
            content: content,
            timestamp: timestamp,
            status: status,
            autoDeleteAt: autoDeleteAt,
            isSent: isSent
        )
    }

    private func upsertConversation(
        id: String,
        participantHash: String,
        lastMessage: String,
        timestamp: Date,
        in ctx: NSManagedObjectContext
    ) throws {
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDConversation")
        request.predicate = NSPredicate(format: "id == %@", id)
        let existing = try? ctx.fetch(request).first

        let obj: NSManagedObject
        if let existing = existing {
            obj = existing
        } else {
            obj = NSManagedObject(
                entity: container.managedObjectModel.entitiesByName["CDConversation"]!,
                insertInto: ctx
            )
            obj.setValue(id,              forKey: "id")
            obj.setValue(participantHash, forKey: "participantHash")
        }

        let encLast = try? encrypt(lastMessage)
        obj.setValue(encLast,   forKey: "lastMessageEncrypted")
        obj.setValue(timestamp, forKey: "lastUpdated")

        if existing == nil {
            obj.setValue(0, forKey: "unreadCount")
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Fetch Conversations
    // -------------------------------------------------------------------------

    public func fetchConversations() throws -> [ConversationModel] {
        let ctx = container.viewContext
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDConversation")
        request.sortDescriptors = [NSSortDescriptor(key: "lastUpdated", ascending: false)]

        let results: [NSManagedObject]
        do {
            results = try ctx.fetch(request)
        } catch {
            throw MessageStoreError.fetchFailed(error)
        }

        let models: [ConversationModel] = results.compactMap { obj in
            guard let id             = obj.value(forKey: "id")              as? String,
                  let participantHash = obj.value(forKey: "participantHash") as? String
            else { return nil }

            let displayName: String
            if let encName = obj.value(forKey: "displayNameEncrypted") as? Data,
               let dec = try? decrypt(encName) {
                displayName = dec
            } else {
                displayName = String(participantHash.prefix(12)) + "..."
            }

            let lastMessage: String
            if let encLast = obj.value(forKey: "lastMessageEncrypted") as? Data,
               let dec = try? decrypt(encLast) {
                lastMessage = dec
            } else {
                lastMessage = ""
            }

            let unreadCount = obj.value(forKey: "unreadCount") as? Int ?? 0
            let lastUpdated = obj.value(forKey: "lastUpdated") as? Date ?? Date()

            return ConversationModel(
                id: id,
                participantHash: participantHash,
                displayName: displayName,
                lastMessage: lastMessage,
                unreadCount: unreadCount,
                lastUpdated: lastUpdated
            )
        }

        DispatchQueue.main.async { [weak self] in
            self?.conversations = models
        }
        return models
    }

    // -------------------------------------------------------------------------
    // MARK: Fetch Messages
    // -------------------------------------------------------------------------

    public func fetchMessages(for conversationId: String) throws -> [MessageModel] {
        let ctx = container.viewContext

        // First, delete any auto-expired messages
        try purgeExpiredMessages(for: conversationId, in: ctx)

        let request = NSFetchRequest<NSManagedObject>(entityName: "CDMessage")
        request.predicate    = NSPredicate(format: "conversationId == %@", conversationId)
        request.sortDescriptors = [NSSortDescriptor(key: "timestamp", ascending: true)]

        let results: [NSManagedObject]
        do {
            results = try ctx.fetch(request)
        } catch {
            throw MessageStoreError.fetchFailed(error)
        }

        return results.compactMap { obj in
            guard let id             = obj.value(forKey: "id")              as? String,
                  let encContent     = obj.value(forKey: "contentEncrypted") as? Data,
                  let timestamp      = obj.value(forKey: "timestamp")        as? Date
            else { return nil }

            let content: String
            do {
                content = try decrypt(encContent)
            } catch {
                return nil
            }

            let statusStr = obj.value(forKey: "status") as? String ?? "sent"
            let status    = MessageStatus(rawValue: statusStr) ?? .sent
            let isSent    = obj.value(forKey: "isSent") as? Bool ?? true
            let autoDeleteAt = obj.value(forKey: "autoDeleteAt") as? Date

            return MessageModel(
                id: id,
                conversationId: conversationId,
                content: content,
                timestamp: timestamp,
                status: status,
                autoDeleteAt: autoDeleteAt,
                isSent: isSent
            )
        }
    }

    // -------------------------------------------------------------------------
    // MARK: Delete
    // -------------------------------------------------------------------------

    public func deleteConversation(id: String) throws {
        let ctx = container.viewContext

        // Delete messages
        let msgRequest = NSFetchRequest<NSManagedObject>(entityName: "CDMessage")
        msgRequest.predicate = NSPredicate(format: "conversationId == %@", id)
        let messages = (try? ctx.fetch(msgRequest)) ?? []
        messages.forEach { ctx.delete($0) }

        // Delete conversation
        let convRequest = NSFetchRequest<NSManagedObject>(entityName: "CDConversation")
        convRequest.predicate = NSPredicate(format: "id == %@", id)
        let conversations = (try? ctx.fetch(convRequest)) ?? []
        conversations.forEach { ctx.delete($0) }

        do {
            try ctx.save()
        } catch {
            throw MessageStoreError.saveFailed(error)
        }

        DispatchQueue.main.async { [weak self] in
            self?.conversations.removeAll { $0.id == id }
        }
    }

    public func deleteMessage(id: String, conversationId: String) throws {
        let ctx = container.viewContext
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDMessage")
        request.predicate = NSPredicate(format: "id == %@", id)
        let results = (try? ctx.fetch(request)) ?? []
        results.forEach { ctx.delete($0) }

        do {
            try ctx.save()
        } catch {
            throw MessageStoreError.saveFailed(error)
        }
    }

    public func updateDisplayName(_ name: String, for conversationId: String) throws {
        let ctx = container.viewContext
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDConversation")
        request.predicate = NSPredicate(format: "id == %@", conversationId)
        guard let obj = try? ctx.fetch(request).first else { return }

        let enc = try encrypt(name)
        obj.setValue(enc, forKey: "displayNameEncrypted")

        do {
            try ctx.save()
        } catch {
            throw MessageStoreError.saveFailed(error)
        }
    }

    public func markAsRead(conversationId: String) throws {
        let ctx = container.viewContext
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDConversation")
        request.predicate = NSPredicate(format: "id == %@", conversationId)
        guard let obj = try? ctx.fetch(request).first else { return }
        obj.setValue(0, forKey: "unreadCount")
        try? ctx.save()
    }

    // -------------------------------------------------------------------------
    // MARK: Auto-delete
    // -------------------------------------------------------------------------

    private func purgeExpiredMessages(for conversationId: String, in ctx: NSManagedObjectContext) throws {
        let now = Date()
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDMessage")
        request.predicate = NSPredicate(
            format: "conversationId == %@ AND autoDeleteAt != nil AND autoDeleteAt <= %@",
            conversationId, now as NSDate
        )
        let expired = (try? ctx.fetch(request)) ?? []
        guard !expired.isEmpty else { return }
        expired.forEach { ctx.delete($0) }
        try ctx.save()
    }

    /// Call periodically (e.g., on app foreground) to purge all expired messages.
    public func purgeAllExpiredMessages() {
        let ctx = container.viewContext
        let now = Date()
        let request = NSFetchRequest<NSManagedObject>(entityName: "CDMessage")
        request.predicate = NSPredicate(
            format: "autoDeleteAt != nil AND autoDeleteAt <= %@", now as NSDate
        )
        let expired = (try? ctx.fetch(request)) ?? []
        guard !expired.isEmpty else { return }
        expired.forEach { ctx.delete($0) }
        try? ctx.save()
    }
}
