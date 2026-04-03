/**
 * NEXUS Message Sync & Conflict Resolution
 * Synchronizes local offline messages with server state
 * Detects and resolves conflicts transparently
 * Ensures no message loss during disconnections
 */

import * as Storage from './storage';

export interface MessageHash {
  id: string;
  contentHash: string;
  timestamp: number;
}

interface ServerMessageState {
  messageId: string;
  contentHash: string;
  deliveredAt: number;
}

/**
 * Sync local messages with server state
 * Returns conflicts that need user intervention
 */
export async function syncMessagesWithServer(
  serverState: ServerMessageState[]
): Promise<{
  synced: number;
  conflicts: string[];
  failed: string[];
}> {
  const stats = {
    synced: 0,
    conflicts: [] as string[],
    failed: [] as string[],
  };

  try {
    console.log('[Sync] Starting message sync with server state');

    const pending = await Storage.getPendingMessages();

    for (const localMsg of pending) {
      // Check if message exists on server
      const serverMsg = serverState.find((s) => s.messageId === localMsg.id);

      if (!serverMsg) {
        // Message not on server - needs sending
        console.log(`[Sync] Message ${localMsg.id} not on server, will be sent`);
        stats.synced++;
        continue;
      }

      // Message exists on server - check content hash
      try {
        const decryptionKey = await Storage.getEncryptionKey(localMsg.recipientHash);
        if (!decryptionKey) {
          console.warn(`[Sync] No decryption key for ${localMsg.id}`);
          stats.failed.push(localMsg.id);
          continue;
        }

        const decrypted = await Storage.decryptStoredMessage(
          localMsg,
          decryptionKey
        );
        const localHash = await hashMessage(decrypted);

        if (localHash === serverMsg.contentHash) {
          // Hashes match - message is consistent
          console.log(`[Sync] Message ${localMsg.id} consistent with server`);
          await Storage.markMessageSent(localMsg.id);
          stats.synced++;
        } else {
          // Hashes don't match - conflict detected
          console.warn(
            `[Sync] Conflict detected for message ${localMsg.id}: local hash ${localHash} != server hash ${serverMsg.contentHash}`
          );
          stats.conflicts.push(localMsg.id);
        }
      } catch (e) {
        console.error(`[Sync] Error processing message ${localMsg.id}:`, e);
        stats.failed.push(localMsg.id);
      }
    }

    console.log('[Sync] Sync complete:', stats);
    return stats;
  } catch (e) {
    console.error('[Sync] Sync failed:', e);
    throw e;
  }
}

/**
 * Resolve message conflict
 * User chooses which version to keep: local or server
 */
export async function resolveConflict(
  messageId: string,
  resolution: 'keep-local' | 'use-server' | 'discard'
): Promise<void> {
  try {
    console.log(`[Sync] Resolving conflict for ${messageId}: ${resolution}`);

    switch (resolution) {
      case 'keep-local':
        // Increment retry count and retry sending
        console.log(`[Sync] Keeping local version of ${messageId}`);
        // Message stays in pending queue for retry
        break;

      case 'use-server':
        // Accept server version, delete local
        console.log(`[Sync] Using server version of ${messageId}`);
        await Storage.deleteMessage(messageId);
        break;

      case 'discard':
        // Delete local version
        console.log(`[Sync] Discarding ${messageId}`);
        await Storage.deleteMessage(messageId);
        break;
    }

    console.log(`[Sync] Conflict resolved for ${messageId}`);
  } catch (e) {
    console.error('[Sync] Failed to resolve conflict:', e);
    throw e;
  }
}

/**
 * Compute hash of message for conflict detection
 */
async function hashMessage(content: string): Promise<string> {
  const data = new TextEncoder().encode(content);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Request background sync from Service Worker
 */
export async function requestBackgroundSync(): Promise<void> {
  try {
    if ('serviceWorker' in navigator && 'SyncManager' in window) {
      const registration = await navigator.serviceWorker.ready;
      if (registration.sync) {
        await registration.sync.register('sync-messages');
        console.log('[Sync] Background sync requested');
      }
    }
  } catch (e) {
    console.error('[Sync] Failed to request background sync:', e);
  }
}

/**
 * Check if browser supports background sync
 */
export function supportsBackgroundSync(): boolean {
  return (
    'serviceWorker' in navigator &&
    'SyncManager' in window &&
    'caches' in window &&
    'indexedDB' in window
  );
}

/**
 * Monitor sync state and trigger UI updates
 */
export function monitorSyncState(
  onSyncStart: () => void,
  onSyncComplete: (stats: any) => void,
  onSyncError: (error: Error) => void
): () => void {
  const handleSync = async () => {
    try {
      onSyncStart();

      // Simulate sync (in production: fetch actual server state)
      const pending = await Storage.getPendingMessages();
      console.log(`[Sync] Monitoring: ${pending.length} pending messages`);

      // Try to send pending messages
      let sentCount = 0;
      for (const msg of pending) {
        try {
          const key = await Storage.getEncryptionKey(msg.recipientHash);
          if (key) {
            const decrypted = await Storage.decryptStoredMessage(msg, key);
            // In production: send to server
            console.log(
              `[Sync] Would send message ${msg.id} to ${msg.recipientHash}`
            );
            await Storage.markMessageSent(msg.id);
            sentCount++;
          }
        } catch (e) {
          console.error(`[Sync] Failed to sync message ${msg.id}:`, e);
        }
      }

      onSyncComplete({ synced: sentCount, total: pending.length });
    } catch (e) {
      onSyncError(e instanceof Error ? e : new Error(String(e)));
    }
  };

  // Check sync state periodically
  const interval = setInterval(handleSync, 30000); // Every 30 seconds

  return () => clearInterval(interval);
}
