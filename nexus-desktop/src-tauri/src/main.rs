// src-tauri/src/main.rs - NEXUS Desktop Client with Tauri
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

mod crypto;
mod storage;
mod relay;

/// Application state
struct AppState {
    crypto: crypto::NexusCrypto,
    storage: Mutex<storage::LocalStorage>,
    relay: Mutex<relay::RelayClient>,
    identity: Mutex<Option<crypto::NexusIdentity>>,
}

#[tauri::command]
async fn generate_identity(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let crypto = &state.crypto;
    let identity = crypto.generate_identity()?;
    let hash = identity.identity_hash.clone();
    *state.identity.lock().await = Some(identity);
    Ok(hash)
}

#[tauri::command]
async fn get_fingerprint(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let identity = state.identity.lock().await;
    match identity.as_ref() {
        Some(id) => Ok(id.fingerprint.clone()),
        None => Err("Identity not generated".into()),
    }
}

#[tauri::command]
async fn send_message(
    content: String,
    recipient_hash: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Save outgoing message
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let msg = storage::StoredMessage {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: recipient_hash.clone(),
        content: content.clone(),
        is_outgoing: true,
        timestamp: now,
        delivered: false,
        read: false,
    };

    {
        let mut storage = state.storage.lock().await;
        storage.save_message(msg)?;
    }

    // Send via relay
    let relay = state.relay.lock().await;
    relay.send_message(recipient_hash, base64_encode(content.as_bytes()), 86400)?;

    Ok("sent".to_string())
}

#[tauri::command]
async fn get_conversations(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let storage = state.storage.lock().await;
    let convs = storage.get_conversations();
    serde_json::to_string(&convs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_messages(
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let storage = state.storage.lock().await;
    let msgs = storage.get_messages(&conversation_id);
    serde_json::to_string(&msgs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_conversation(
    participant_hash: String,
    display_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut storage = state.storage.lock().await;
    let conv = storage.create_conversation(participant_hash, display_name)?;
    serde_json::to_string(&conv).map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_as_read(
    conversation_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut storage = state.storage.lock().await;
    storage.mark_as_read(&conversation_id)
}

#[tauri::command]
async fn connect_to_relay(
    url: String,
    recipient_hash: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut relay = state.relay.lock().await;
    *relay = relay::RelayClient::new(&url);

    // Clone what we need for the message handler
    let storage_clone = state.storage.clone();

    relay.connect(recipient_hash, move |msg| {
        // Handle incoming messages
        match msg {
            relay::ServerMessage::Deliver { sealed_content, received_at } => {
                // Decode and save message
                if let Ok(content) = base64_decode(&sealed_content) {
                    if let Ok(content_str) = String::from_utf8(content) {
                        let message = storage::StoredMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            conversation_id: "unknown".to_string(),
                            content: content_str,
                            is_outgoing: false,
                            timestamp: received_at,
                            delivered: true,
                            read: false,
                        };
                        // Note: In production, we'd properly handle this async
                        let _ = storage_clone.try_lock().map(|mut s| s.save_message(message));
                    }
                }
            }
            relay::ServerMessage::OfflineMessages { messages } => {
                for offline_msg in messages {
                    if let Ok(content) = base64_decode(&offline_msg.sealed_content) {
                        if let Ok(content_str) = String::from_utf8(content) {
                            let message = storage::StoredMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                conversation_id: "unknown".to_string(),
                                content: content_str,
                                is_outgoing: false,
                                timestamp: offline_msg.received_at,
                                delivered: true,
                                read: false,
                            };
                            let _ = storage_clone.try_lock().map(|mut s| s.save_message(message));
                        }
                    }
                }
            }
            relay::ServerMessage::Error { code, message } => {
                eprintln!("Relay error {}: {}", code, message);
            }
            _ => {}
        }
    }).await
}

#[tauri::command]
async fn disconnect_from_relay(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut relay = state.relay.lock().await;
    relay.disconnect();
    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(data).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize app state
            let storage = storage::LocalStorage::new()?;
            let relay = relay::RelayClient::new("ws://localhost:8443/ws");

            app.manage(AppState {
                crypto: crypto::NexusCrypto::new(),
                storage: Mutex::new(storage),
                relay: Mutex::new(relay),
                identity: Mutex::new(None),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_identity,
            get_fingerprint,
            send_message,
            get_conversations,
            get_messages,
            create_conversation,
            mark_as_read,
            connect_to_relay,
            disconnect_from_relay,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            _ => {}
        });
}
