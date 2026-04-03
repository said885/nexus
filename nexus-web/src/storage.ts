/**
 * NEXUS Encrypted Storage
 * IndexedDB with AES-256-GCM encryption for offline messages
 * Messages are encrypted locally before persistence
 * No plaintext stored on disk
 */

import * as Crypto from './crypto';

const DB_NAME = 'nexus-sync';
const DB_VERSION = 1;

interface StoredMessage {
  id: string;
  recipientHash: string;
  encryptedContent: string;
  iv: string;
  timestamp: number;
  status: 'pending' | 'sent' | 'delivered' | 'failed';
  retryCount: number;
}

interface StoredIdentity {
  hash: string;
  publicKey: string;
  privateKey: string;
  createdAt: number;
}

let db: IDBDatabase | null = null;

/**
 * Initialize encrypted storage database
 */
export async function initializeStorage(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onerror = () => {
      console.error('Failed to open IndexedDB:', request.error);
      reject(request.error);
    };

    request.onsuccess = () => {
      db = request.result;
      console.log('[Storage] Initialized IndexedDB');
      resolve(db);
    };

    request.onupgradeneeded = (event) => {
      const database = (event.target as IDBOpenDBRequest).result;

      // Create object stores
      if (!database.objectStoreNames.contains('outgoingMessages')) {
        const msgStore = database.createObjectStore('outgoingMessages', {
          keyPath: 'id',
        });
        msgStore.createIndex('status', 'status', { unique: false });
        msgStore.createIndex('recipientHash', 'recipientHash', { unique: false });
        msgStore.createIndex('timestamp', 'timestamp', { unique: false });
        console.log('[Storage] Created outgoingMessages store');
      }

      if (!database.objectStoreNames.contains('identity')) {
        database.createObjectStore('identity', { keyPath: 'hash' });
        console.log('[Storage] Created identity store');
      }

      if (!database.objectStoreNames.contains('decryptionKeys')) {
        database.createObjectStore('decryptionKeys', { keyPath: 'recipientHash' });
        console.log('[Storage] Created decryptionKeys store');
      }
    };
  });
}

/**
 * Store outgoing message encrypted locally
 * Message is encrypted with AES-256-GCM before persistence
 */
export async function storeOutgoingMessage(
  message: {
    id: string;
    recipientHash: string;
    content: string;
    timestamp: number;
  },
  encryptionKey: CryptoKey
): Promise<void> {
  if (!db) await initializeStorage();

  try {
    // Encrypt message content
    const encrypted = await Crypto.encryptMessage(message.content, encryptionKey);

    const storedMessage: StoredMessage = {
      id: message.id,
      recipientHash: message.recipientHash,
      encryptedContent: encrypted.ciphertext,
      iv: encrypted.iv,
      timestamp: message.timestamp,
      status: 'pending',
      retryCount: 0,
    };

    await new Promise<void>((resolve, reject) => {
      const transaction = db!.transaction('outgoingMessages', 'readwrite');
      const store = transaction.objectStore('outgoingMessages');
      const request = store.add(storedMessage);

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        console.log(`[Storage] Stored encrypted message ${message.id}`);
        resolve();
      };
    });
  } catch (e) {
    console.error('[Storage] Failed to store message:', e);
    throw e;
  }
}

/**
 * Retrieve pending messages for sending
 */
export async function getPendingMessages(): Promise<StoredMessage[]> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction('outgoingMessages', 'readonly');
    const store = transaction.objectStore('outgoingMessages');
    const index = store.index('status');
    const request = index.getAll('pending');

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      console.log(`[Storage] Retrieved ${request.result.length} pending messages`);
      resolve(request.result);
    };
  });
}

/**
 * Decrypt stored message for sending
 */
export async function decryptStoredMessage(
  stored: StoredMessage,
  encryptionKey: CryptoKey
): Promise<string> {
  try {
    return await Crypto.decryptMessage(
      {
        iv: stored.iv,
        ciphertext: stored.encryptedContent,
      },
      encryptionKey
    );
  } catch (e) {
    console.error(`[Storage] Failed to decrypt message ${stored.id}:`, e);
    throw e;
  }
}

/**
 * Mark message as sent
 */
export async function markMessageSent(messageId: string): Promise<void> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction('outgoingMessages', 'readwrite');
    const store = transaction.objectStore('outgoingMessages');
    const request = store.get(messageId);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const message = request.result as StoredMessage;
      if (message) {
        message.status = 'sent';
        const updateRequest = store.put(message);
        updateRequest.onerror = () => reject(updateRequest.error);
        updateRequest.onsuccess = () => {
          console.log(`[Storage] Marked message ${messageId} as sent`);
          resolve();
        };
      }
    };
  });
}

/**
 * Delete message after successful delivery
 */
export async function deleteMessage(messageId: string): Promise<void> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction('outgoingMessages', 'readwrite');
    const store = transaction.objectStore('outgoingMessages');
    const request = store.delete(messageId);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      console.log(`[Storage] Deleted message ${messageId}`);
      resolve();
    };
  });
}

/**
 * Store encryption key for recipient
 */
export async function storeEncryptionKey(
  recipientHash: string,
  key: CryptoKey
): Promise<void> {
  if (!db) await initializeStorage();

  try {
    // Export key to JSON for storage
    const exported = await crypto.subtle.exportKey('jwk', key);

    await new Promise<void>((resolve, reject) => {
      const transaction = db!.transaction('decryptionKeys', 'readwrite');
      const store = transaction.objectStore('decryptionKeys');
      const request = store.put({
        recipientHash,
        keyData: JSON.stringify(exported),
        timestamp: Date.now(),
      });

      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        console.log(`[Storage] Stored encryption key for ${recipientHash}`);
        resolve();
      };
    });
  } catch (e) {
    console.error('[Storage] Failed to store encryption key:', e);
    throw e;
  }
}

/**
 * Retrieve encryption key for recipient
 */
export async function getEncryptionKey(recipientHash: string): Promise<CryptoKey | null> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction('decryptionKeys', 'readonly');
    const store = transaction.objectStore('decryptionKeys');
    const request = store.get(recipientHash);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const record = request.result;
      if (!record) {
        resolve(null);
        return;
      }

      // Re-import key from stored JSON
      try {
        const keyData = JSON.parse(record.keyData);
        crypto.subtle
          .importKey('jwk', keyData, { name: 'AES-GCM' }, true, ['encrypt', 'decrypt'])
          .then((key) => {
            console.log(`[Storage] Retrieved encryption key for ${recipientHash}`);
            resolve(key);
          })
          .catch((e) => {
            console.error('[Storage] Failed to import key:', e);
            reject(e);
          });
      } catch (e) {
        console.error('[Storage] Failed to parse stored key:', e);
        reject(e);
      }
    };
  });
}

/**
 * Clear all storage (for logout)
 */
export async function clearAllStorage(): Promise<void> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction(
      ['outgoingMessages', 'decryptionKeys'],
      'readwrite'
    );

    transaction.objectStore('outgoingMessages').clear();
    transaction.objectStore('decryptionKeys').clear();

    transaction.onerror = () => reject(transaction.error);
    transaction.oncomplete = () => {
      console.log('[Storage] Cleared all local storage');
      resolve();
    };
  });
}

/**
 * Get storage statistics
 */
export async function getStorageStats(): Promise<{
  pendingMessages: number;
  totalSize: number;
}> {
  if (!db) await initializeStorage();

  return new Promise((resolve, reject) => {
    const transaction = db!.transaction('outgoingMessages', 'readonly');
    const store = transaction.objectStore('outgoingMessages');
    const request = store.getAll();

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const messages = request.result as StoredMessage[];
      let totalSize = 0;

      messages.forEach((msg) => {
        totalSize +=
          msg.encryptedContent.length +
          msg.iv.length +
          msg.recipientHash.length;
      });

      const pending = messages.filter((m) => m.status === 'pending').length;

      resolve({
        pendingMessages: pending,
        totalSize,
      });
    };
  });
}
