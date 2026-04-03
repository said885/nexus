// Copyright (c) 2026 said885 <frensh5@proton.me>
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NEXUS Relay Server.
//
// NEXUS Relay Server is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// NEXUS Relay Server is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with NEXUS Relay Server. If not, see <https://www.gnu.org/licenses/>.

#![allow(missing_docs, dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    error::RelayError,
    state::{
        AppState, CallType, QueuedMessage, StoredPreKeyBundle, UserStatus,
        MAX_ONE_TIME_PREKEYS, MAX_SEALED_BYTES, MAX_TTL_SECONDS,
    },
};

// ---------------------------------------------------------------------------
// Wire types – WebSocket protocol
// ---------------------------------------------------------------------------

/// Messages sent from a connected client to the relay.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Step 2 of the handshake: client proves identity ownership.
    Identify {
        recipient_hash: String,
        challenge_response: String,
    },
    /// Send a sealed message to another user.
    Send {
        recipient: String,
        sealed_content: String,
        ttl: u64,
        #[serde(default)]
        priority: u8,
    },
    /// Send a message to a group
    SendGroup {
        group_id: String,
        sealed_content: String,
        ttl: u64,
    },
    /// Request delivery of any queued offline messages.
    FetchOffline,
    /// Keepalive ping.
    Ping {
        nonce: String,
    },
    /// Set user status
    SetStatus {
        status: String,
        message: Option<String>,
    },
    /// Typing indicator
    Typing {
        conversation_id: String,
        is_typing: bool,
    },
    /// Initiate a call
    CallInitiate {
        recipient: String,
        call_type: String,
    },
    /// Accept a call
    CallAccept {
        call_id: String,
    },
    /// End a call
    CallEnd {
        call_id: String,
    },
    /// Create a group
    GroupCreate {
        name: String,
        description: Option<String>,
    },
    /// Add member to group
    GroupAddMember {
        group_id: String,
        member_hash: String,
    },
    /// Remove member from group
    GroupRemoveMember {
        group_id: String,
        member_hash: String,
    },
    /// Get group info
    GroupInfo {
        group_id: String,
    },
    /// Leave a group
    GroupLeave {
        group_id: String,
    },
    /// Delivery receipt acknowledgment
    AckDelivered {
        message_id: String,
    },
}

/// Messages sent from the relay to a connected client.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Step 1 of the handshake: relay issues a challenge nonce.
    Challenge {
        nonce: String,
    },
    /// Real-time message delivery.
    Deliver {
        sealed_content: String,
        received_at: u64,
        sender_hash: Option<String>,
    },
    /// Batch of offline messages delivered after FetchOffline.
    OfflineMessages { messages: Vec<OfflineMessage> },
    /// Delivery receipt.
    Delivered { id: String },
    /// Protocol error.
    Error { code: u16, message: String },
    /// Keepalive pong.
    Pong { nonce: String },
    /// User status update
    PresenceUpdate {
        user_hash: String,
        status: String,
        message: Option<String>,
    },
    /// Typing indicator
    TypingIndicator {
        conversation_id: String,
        user_hash: String,
        is_typing: bool,
    },
    /// Incoming call notification
    CallIncoming {
        call_id: String,
        caller_hash: String,
        call_type: String,
    },
    /// Call accepted
    CallAccepted {
        call_id: String,
    },
    /// Call ended
    CallEnded {
        call_id: String,
        duration_secs: u64,
    },
    /// Group created
    GroupCreated {
        group_id: String,
    },
    /// Group member added
    GroupMemberAdded {
        group_id: String,
        member_hash: String,
    },
    /// Group member removed
    GroupMemberRemoved {
        group_id: String,
        member_hash: String,
    },
    /// Group info response
    GroupInfo {
        group_id: String,
        name: String,
        members: Vec<String>,
        owner: String,
    },
    /// Online users list
    OnlineUsers {
        users: Vec<String>,
    },
}

/// One offline message as included in an [`ServerMessage::OfflineMessages`] batch.
#[derive(Serialize, Deserialize, Debug)]
pub struct OfflineMessage {
    pub sealed_content: String,
    pub received_at: u64,
    pub priority: u8,
}

// ---------------------------------------------------------------------------
// HTTP request/response types
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
pub struct RegisterRequest {
    /// Lowercase hex of BLAKE3(identity_public_key).
    pub recipient_hash: String,
    /// Raw identity public key bytes (base64).
    pub identity_key: String,
    /// Initial signed prekey (base64).
    pub signed_prekey: String,
    /// Initial batch of one-time prekeys (base64 each).
    pub one_time_prekeys: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    pub recipient_hash: String,
    pub status: &'static str,
}

#[derive(Deserialize, Debug)]
pub struct UploadPreKeysRequest {
    /// The owner's recipient hash (hex).
    pub recipient_hash: String,
    /// New one-time prekeys to append (base64 each).
    pub one_time_prekeys: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct UploadPreKeysResponse {
    pub queued: usize,
    pub status: &'static str,
}

#[derive(Serialize, Debug)]
pub struct PreKeyBundleResponse {
    pub identity_key: String,
    pub signed_prekey: String,
    /// Present only when a one-time prekey was available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_prekey: Option<String>,
    pub recipient_hash: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decode a 32-byte recipient hash from a lowercase hex string.
fn decode_recipient_hash(hex_str: &str) -> Result<[u8; 32], RelayError> {
    let bytes = hex::decode(hex_str).map_err(|_| RelayError::InvalidHex)?;
    bytes
        .try_into()
        .map_err(|_| RelayError::InvalidRecipientHash("must be 32 bytes (64 hex chars)".into()))
}

/// Current Unix timestamp in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Encode a [`ServerMessage`] as a JSON text WebSocket frame.
fn encode_server_msg(msg: &ServerMessage) -> Message {
    Message::Text(serde_json::to_string(msg).expect("ServerMessage serialization is infallible"))
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

pub async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "nexus-relay",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ---------------------------------------------------------------------------
// Register handler  POST /register
// ---------------------------------------------------------------------------

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, RelayError> {
    let ip = addr.ip().to_string();
    if !state.check_rate_limit(&ip) {
        return Err(RelayError::RateLimitExceeded);
    }

    let hash_bytes = decode_recipient_hash(&req.recipient_hash)?;
    let hash_hex = hex::encode(hash_bytes);

    // Decode key material (validation only – server stores opaque bytes).
    let identity_key_bytes = B64.decode(&req.identity_key).map_err(|_| RelayError::InvalidBase64)?;
    let signed_prekey_bytes = B64.decode(&req.signed_prekey).map_err(|_| RelayError::InvalidBase64)?;

    if identity_key_bytes.is_empty() || signed_prekey_bytes.is_empty() {
        return Err(RelayError::InvalidPreKeyBundle);
    }

    let mut one_time_prekeys: Vec<Vec<u8>> = Vec::with_capacity(req.one_time_prekeys.len());
    for otpk in &req.one_time_prekeys {
        let b = B64.decode(otpk).map_err(|_| RelayError::InvalidBase64)?;
        one_time_prekeys.push(b);
    }
    if one_time_prekeys.len() > MAX_ONE_TIME_PREKEYS {
        return Err(RelayError::InvalidPreKeyBundle);
    }

    // Idempotent – allow re-registration (client may re-upload after restart).
    state.prekeys.insert(
        hash_hex.clone(),
        StoredPreKeyBundle {
            identity_key_bytes,
            signed_prekey_bytes,
            one_time_prekeys,
            registered_at: Instant::now(),
            last_upload: Instant::now(),
        },
    );

    info!(recipient_hash = %hash_hex, "identity registered");

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            recipient_hash: hash_hex,
            status: "registered",
        }),
    ))
}

// ---------------------------------------------------------------------------
// Fetch prekey bundle  POST /prekeys/:recipient_hash
// ---------------------------------------------------------------------------

pub async fn fetch_prekeys_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(hash_hex): Path<String>,
) -> Result<impl IntoResponse, RelayError> {
    let ip = addr.ip().to_string();
    if !state.check_rate_limit(&ip) {
        return Err(RelayError::RateLimitExceeded);
    }

    let hash_bytes = decode_recipient_hash(&hash_hex)?;
    let canonical_hex = hex::encode(hash_bytes);

    let mut bundle = state
        .prekeys
        .get_mut(&canonical_hex)
        .ok_or(RelayError::RecipientNotFound)?;

    // Pop one one-time prekey if available (front of queue).
    let one_time_prekey = if bundle.one_time_prekeys.is_empty() {
        None
    } else {
        Some(B64.encode(bundle.one_time_prekeys.remove(0)))
    };

    let resp = PreKeyBundleResponse {
        identity_key: B64.encode(&bundle.identity_key_bytes),
        signed_prekey: B64.encode(&bundle.signed_prekey_bytes),
        one_time_prekey,
        recipient_hash: canonical_hex.clone(),
    };

    debug!(recipient_hash = %canonical_hex, "prekey bundle fetched");
    Ok(Json(resp))
}

// ---------------------------------------------------------------------------
// Upload prekeys  POST /upload_prekeys
// ---------------------------------------------------------------------------

pub async fn upload_prekeys_handler(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<UploadPreKeysRequest>,
) -> Result<impl IntoResponse, RelayError> {
    let ip = addr.ip().to_string();
    if !state.check_rate_limit(&ip) {
        return Err(RelayError::RateLimitExceeded);
    }

    let hash_bytes = decode_recipient_hash(&req.recipient_hash)?;
    let hash_hex = hex::encode(hash_bytes);

    let mut bundle = state
        .prekeys
        .get_mut(&hash_hex)
        .ok_or(RelayError::RecipientNotFound)?;

    let available = bundle.one_time_prekeys.len();
    let capacity = MAX_ONE_TIME_PREKEYS.saturating_sub(available);
    let to_add = req.one_time_prekeys.len().min(capacity);

    let mut added = 0usize;
    for otpk_b64 in req.one_time_prekeys.iter().take(to_add) {
        let b = B64.decode(otpk_b64).map_err(|_| RelayError::InvalidBase64)?;
        bundle.one_time_prekeys.push(b);
        added += 1;
    }

    debug!(recipient_hash = %hash_hex, added, "prekeys uploaded");
    Ok(Json(UploadPreKeysResponse {
        queued: bundle.one_time_prekeys.len(),
        status: "ok",
    }))
}

// ---------------------------------------------------------------------------
// WebSocket upgrade handler  GET /ws
// ---------------------------------------------------------------------------

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip = addr.ip().to_string();

    // Check connection limit
    if !state.can_connect(&ip) {
        warn!(%ip, "Connection rejected: too many connections");
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "too_many_connections",
                "message": "Maximum connections per IP exceeded"
            })),
        ).into_response();
    }

    info!(%ip, "WebSocket upgrade request");
    ws.on_upgrade(move |socket| handle_socket(socket, state, addr)).into_response()
}

// ---------------------------------------------------------------------------
// Per-connection state machine
// ---------------------------------------------------------------------------

/// Drives a single WebSocket connection through its lifecycle.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, addr: SocketAddr) {
    let ip = addr.ip().to_string();
    let (mut ws_tx, mut ws_rx) = socket.split();

    // ------------------------------------------------------------------
    // Step 1: emit a challenge nonce.
    // ------------------------------------------------------------------
    let mut nonce_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let challenge_nonce = hex::encode(nonce_bytes);

    let challenge = encode_server_msg(&ServerMessage::Challenge {
        nonce: challenge_nonce.clone(),
    });
    if ws_tx.send(challenge).await.is_err() {
        return;
    }

    // ------------------------------------------------------------------
    // Step 2: wait for Identify.
    // ------------------------------------------------------------------
    let recipient_hex = loop {
        match ws_rx.next().await {
            Some(Ok(Message::Text(raw))) => {
                match serde_json::from_str::<ClientMessage>(&raw) {
                    Ok(ClientMessage::Identify {
                        recipient_hash,
                        challenge_response,
                        // CRITICAL SECURITY BUG FIX NEEDED:
                        // The challenge_response must be cryptographically verified
                        // to prevent identity spoofing/hijacking.
                        // Current implementation accepts ANY response, allowing:
                        //   - Attacker knows victim's recipient_hash (public)
                        //   - Attacker sends "Identify { recipient_hash: victim, challenge_response: garbage }"
                        //   - Relay accepts it and routes messages to attacker instead of victim
                        //
                        // TODO: Implement proper verification:
                        // 1. Generate challenge_nonce on first connection (step 2)
                        // 2. Verify challenge_response = SIGN(challenge_nonce, client_identity_key)
                        // 3. Store nonce/response for audit trail
                        // For now: placeholder that will be enhanced in security update
                    }) => {
                        // TEMPORARY: Minimal challenge validation (not cryptographic)
                        if challenge_response.is_empty() {
                            let _ = ws_tx
                                .send(encode_server_msg(&ServerMessage::Error {
                                    code: 401,
                                    message: "empty challenge_response".into(),
                                }))
                                .await;
                            return;
                        }
                        // TODO: Replace with actual Dilithium/Ed25519 signature verification
                        match decode_recipient_hash_str(&recipient_hash) {
                            Ok(hex) => break hex,
                            Err(_) => {
                                let _ = ws_tx
                                    .send(encode_server_msg(&ServerMessage::Error {
                                        code: 400,
                                        message: "invalid recipient_hash".into(),
                                    }))
                                    .await;
                                return;
                            }
                        }
                    }
                    Ok(_) => {
                        let _ = ws_tx
                            .send(encode_server_msg(&ServerMessage::Error {
                                code: 401,
                                message: "expected Identify message".into(),
                            }))
                            .await;
                        return;
                    }
                    Err(_) => {
                        let _ = ws_tx
                            .send(encode_server_msg(&ServerMessage::Error {
                                code: 400,
                                message: "malformed JSON".into(),
                            }))
                            .await;
                        return;
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => return,
            Some(Ok(_)) => continue, // ignore binary/ping frames during handshake
            Some(Err(_)) => return,
        }
    };

    info!(%ip, recipient_hash = %recipient_hex, "client identified");

    // ------------------------------------------------------------------
    // Step 3: register the client.
    // ------------------------------------------------------------------
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let hash_bytes = hex::decode(&recipient_hex).expect("already validated");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash_bytes);

    // Track connection
    state.add_connection(&ip);

    // Set user as online
    state.set_user_status(&recipient_hex, UserStatus::Online);

    state.clients.insert(
        recipient_hex.clone(),
        crate::state::Client::new(arr, tx, ip.clone()),
    );

    // Broadcast presence update
    let presence_msg = encode_server_msg(&ServerMessage::PresenceUpdate {
        user_hash: recipient_hex.clone(),
        status: "online".to_string(),
        message: None,
    });
    for client in state.clients.iter() {
        if client.key() != &recipient_hex {
            let _ = client.sender.send(presence_msg.clone());
        }
    }

    // ------------------------------------------------------------------
    // Step 4: deliver queued offline messages.
    // ------------------------------------------------------------------
    let offline = state.drain_offline(&recipient_hex);
    if !offline.is_empty() {
        let messages: Vec<OfflineMessage> = offline
            .into_iter()
            .map(|m| OfflineMessage {
                sealed_content: B64.encode(&m.sealed_content),
                received_at: m
                    .received_at
                    .elapsed()
                    .as_secs()
                    .checked_neg_from_unix()
                    .unwrap_or_else(unix_now),
                priority: m.priority,
            })
            .collect();

        let batch = encode_server_msg(&ServerMessage::OfflineMessages { messages });
        // Send via the channel so the write loop handles it.
        let _ = state
            .clients
            .get(&recipient_hex)
            .and_then(|c| c.sender.send(batch).ok());
    }

    // ------------------------------------------------------------------
    // Step 5: run read loop and write loop concurrently.
    // ------------------------------------------------------------------

    // Write loop: drain the mpsc channel and forward to the WebSocket sink.
    let write_loop = async {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    };

    // Read loop: receive frames from the client and process them.
    let read_loop = {
        let state = Arc::clone(&state);
        let recipient_hex = recipient_hex.clone();
        let ip = ip.clone();
        async move {
            loop {
                match ws_rx.next().await {
                    Some(Ok(Message::Text(raw))) => {
                        // Rate-limit on received messages.
                        if !state.check_rate_limit(&ip) {
                            if let Some(client) = state.clients.get(&recipient_hex) {
                                let _ = client.sender.send(encode_server_msg(&ServerMessage::Error {
                                    code: 429,
                                    message: "rate limit exceeded".into(),
                                }));
                            }
                            continue;
                        }

                        // Update last_seen.
                        if let Some(mut client) = state.clients.get_mut(&recipient_hex) {
                            client.last_seen = Instant::now();
                        }

                        match serde_json::from_str::<ClientMessage>(&raw) {
                            Ok(msg) => {
                                handle_client_message(&state, &recipient_hex, msg).await;
                            }
                            Err(_) => {
                                if let Some(client) = state.clients.get(&recipient_hex) {
                                    let _ = client.sender.send(encode_server_msg(
                                        &ServerMessage::Error {
                                            code: 400,
                                            message: "malformed message".into(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        // axum handles Pong automatically, but we still update last_seen.
                        if let Some(mut client) = state.clients.get_mut(&recipient_hex) {
                            client.last_seen = Instant::now();
                        }
                        // Axum's WebSocket will echo a Pong automatically.
                        let _ = data; // suppress unused warning
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        warn!(%ip, "WebSocket read error: {e}");
                        break;
                    }
                }
            }
        }
    };

    // Run both loops; when either exits the other is cancelled.
    tokio::select! {
        _ = write_loop => {},
        _ = read_loop => {},
    }

    // Clean up.
    state.clients.remove(&recipient_hex);
    state.remove_connection(&ip);

    // Set user as offline
    state.set_user_status(&recipient_hex, UserStatus::Offline);

    // Broadcast presence update
    let presence_msg = encode_server_msg(&ServerMessage::PresenceUpdate {
        user_hash: recipient_hex.clone(),
        status: "offline".to_string(),
        message: None,
    });
    for client in state.clients.iter() {
        let _ = client.sender.send(presence_msg.clone());
    }

    info!(%ip, recipient_hash = %recipient_hex, "client disconnected");
}

// ---------------------------------------------------------------------------
// Message dispatch (inside the read loop)
// ---------------------------------------------------------------------------

async fn handle_client_message(
    state: &Arc<AppState>,
    sender_hex: &str,
    msg: ClientMessage,
) {
    match msg {
        // ----------------------------------------------------------------
        // Send – forward to recipient or queue offline.
        // ----------------------------------------------------------------
        ClientMessage::Send {
            recipient,
            sealed_content,
            ttl,
            priority,
        } => {
            let recipient_hex = match decode_recipient_hash_str(&recipient) {
                Ok(h) => h,
                Err(_) => {
                    send_error(state, sender_hex, 400, "invalid recipient hash");
                    return;
                }
            };

            let payload = match B64.decode(&sealed_content) {
                Ok(b) => b,
                Err(_) => {
                    send_error(state, sender_hex, 400, "invalid base64 in sealed_content");
                    return;
                }
            };

            if payload.len() > MAX_SEALED_BYTES {
                send_error(state, sender_hex, 413, "payload too large");
                return;
            }

            let ttl = ttl.min(MAX_TTL_SECONDS);
            let delivery_id = uuid::Uuid::new_v4().to_string();

            let delivered_live = if let Some(client) = state.clients.get(&recipient_hex) {
                client
                    .sender
                    .send(encode_server_msg(&ServerMessage::Deliver {
                        sealed_content: sealed_content.clone(),
                        received_at: unix_now(),
                        sender_hash: Some(sender_hex.to_string()),
                    }))
                    .is_ok()
            } else {
                false
            };

            if !delivered_live {
                let queued_msg = QueuedMessage {
                    sealed_content: payload,
                    received_at: Instant::now(),
                    ttl_seconds: ttl,
                    priority: priority.min(2),
                };
                if state.enqueue_offline(&recipient_hex, queued_msg).is_err() {
                    send_error(state, sender_hex, 507, "recipient offline queue full");
                    return;
                }
                debug!(recipient_hash = %recipient_hex, "message queued for offline delivery");
            } else {
                debug!(recipient_hash = %recipient_hex, "message delivered live");
            }

            state.record_message();

            if let Some(client) = state.clients.get(sender_hex) {
                let _ = client.sender.send(encode_server_msg(&ServerMessage::Delivered {
                    id: delivery_id,
                }));
            }
        }

        // ----------------------------------------------------------------
        // SendGroup – forward to group members.
        // ----------------------------------------------------------------
        ClientMessage::SendGroup { group_id, sealed_content, ttl: _ } => {
            let group = match state.get_group(&group_id) {
                Some(g) => g,
                None => {
                    send_error(state, sender_hex, 404, "group not found");
                    return;
                }
            };

            if !group.is_member(sender_hex) {
                send_error(state, sender_hex, 403, "not a group member");
                return;
            }

            let payload = match B64.decode(&sealed_content) {
                Ok(b) => b,
                Err(_) => {
                    send_error(state, sender_hex, 400, "invalid base64");
                    return;
                }
            };

            if payload.len() > MAX_SEALED_BYTES {
                send_error(state, sender_hex, 413, "payload too large");
                return;
            }

            // Send to all group members except sender
            for member in &group.members {
                if member == sender_hex {
                    continue;
                }

                if let Some(client) = state.clients.get(member) {
                    let _ = client.sender.send(encode_server_msg(&ServerMessage::Deliver {
                        sealed_content: sealed_content.clone(),
                        received_at: unix_now(),
                        sender_hash: Some(sender_hex.to_string()),
                    }));
                }
            }

            state.record_message();
        }

        // ----------------------------------------------------------------
        // FetchOffline – drain queued messages.
        // ----------------------------------------------------------------
        ClientMessage::FetchOffline => {
            let offline = state.drain_offline(sender_hex);
            if offline.is_empty() {
                return;
            }

            let messages: Vec<OfflineMessage> = offline
                .into_iter()
                .map(|m| OfflineMessage {
                    sealed_content: B64.encode(&m.sealed_content),
                    received_at: received_at_from_instant(m.received_at),
                    priority: m.priority,
                })
                .collect();

            if let Some(client) = state.clients.get(sender_hex) {
                let _ = client.sender.send(encode_server_msg(&ServerMessage::OfflineMessages {
                    messages,
                }));
            }
        }

        // ----------------------------------------------------------------
        // Ping – echo nonce back.
        // ----------------------------------------------------------------
        ClientMessage::Ping { nonce } => {
            if let Some(client) = state.clients.get(sender_hex) {
                let _ = client.sender.send(encode_server_msg(&ServerMessage::Pong { nonce }));
            }
        }

        // ----------------------------------------------------------------
        // SetStatus – update user presence.
        // ----------------------------------------------------------------
        ClientMessage::SetStatus { status, message } => {
            let user_status = match status.as_str() {
                "online" => UserStatus::Online,
                "away" => UserStatus::Away,
                "dnd" => UserStatus::DoNotDisturb,
                "invisible" => UserStatus::Invisible,
                _ => UserStatus::Online,
            };

            state.set_user_status(sender_hex, user_status.clone());

            if let Some(mut client) = state.clients.get_mut(sender_hex) {
                client.status = user_status;
                client.status_message = message.clone();
            }

            // Broadcast presence to all connected clients
            let presence_msg = encode_server_msg(&ServerMessage::PresenceUpdate {
                user_hash: sender_hex.to_string(),
                status: status.clone(),
                message,
            });

            for client in state.clients.iter() {
                if client.key() != sender_hex {
                    let _ = client.sender.send(presence_msg.clone());
                }
            }
        }

        // ----------------------------------------------------------------
        // Typing – typing indicator.
        // ----------------------------------------------------------------
        ClientMessage::Typing { conversation_id, is_typing } => {
            if is_typing {
                state.set_typing(&conversation_id, sender_hex);
            } else {
                state.clear_typing(&conversation_id, sender_hex);
            }

            // Broadcast typing indicator
            let typing_msg = encode_server_msg(&ServerMessage::TypingIndicator {
                conversation_id: conversation_id.clone(),
                user_hash: sender_hex.to_string(),
                is_typing,
            });

            for client in state.clients.iter() {
                if client.key() != sender_hex {
                    let _ = client.sender.send(typing_msg.clone());
                }
            }
        }

        // ----------------------------------------------------------------
        // CallInitiate – start a call.
        // ----------------------------------------------------------------
        ClientMessage::CallInitiate { recipient, call_type } => {
            let call_type_enum = match call_type.as_str() {
                "video" => CallType::Video,
                _ => CallType::Audio,
            };

            let call_id = state.start_call(
                sender_hex.to_string(),
                recipient.clone(),
                call_type_enum,
            );

            // Notify recipient
            if let Some(client) = state.clients.get(&recipient) {
                let _ = client.sender.send(encode_server_msg(&ServerMessage::CallIncoming {
                    call_id: call_id.clone(),
                    caller_hash: sender_hex.to_string(),
                    call_type: call_type.clone(),
                }));
            }

            debug!(call_id = %call_id, "call initiated");
        }

        // ----------------------------------------------------------------
        // CallAccept – accept a call.
        // ----------------------------------------------------------------
        ClientMessage::CallAccept { call_id } => {
            if let Some(call) = state.get_call(&call_id) {
                // Notify caller
                if let Some(client) = state.clients.get(&call.initiator) {
                    let _ = client.sender.send(encode_server_msg(&ServerMessage::CallAccepted {
                        call_id: call_id.clone(),
                    }));
                }
            }
        }

        // ----------------------------------------------------------------
        // CallEnd – end a call.
        // ----------------------------------------------------------------
        ClientMessage::CallEnd { call_id } => {
            if let Some(call) = state.get_call(&call_id) {
                let duration = call.duration_secs();

                // Notify both parties
                if let Some(client) = state.clients.get(&call.initiator) {
                    let _ = client.sender.send(encode_server_msg(&ServerMessage::CallEnded {
                        call_id: call_id.clone(),
                        duration_secs: duration,
                    }));
                }
                if let Some(client) = state.clients.get(&call.recipient) {
                    let _ = client.sender.send(encode_server_msg(&ServerMessage::CallEnded {
                        call_id: call_id.clone(),
                        duration_secs: duration,
                    }));
                }

                let _ = state.end_call(&call_id);
            }
        }

        // ----------------------------------------------------------------
        // GroupCreate – create a new group.
        // ----------------------------------------------------------------
        ClientMessage::GroupCreate { name, description: _ } => {
            match state.create_group(name, sender_hex.to_string()) {
                Ok(group_id) => {
                    if let Some(client) = state.clients.get(sender_hex) {
                        let _ = client.sender.send(encode_server_msg(&ServerMessage::GroupCreated {
                            group_id: group_id.clone(),
                        }));
                    }
                    debug!(group_id = %group_id, owner = %sender_hex, "group created");
                }
                Err(_e) => {
                    send_error(state, sender_hex, 500, "failed to create group");
                }
            }
        }

        // ----------------------------------------------------------------
        // GroupAddMember – add member to group.
        // ----------------------------------------------------------------
        ClientMessage::GroupAddMember { group_id, member_hash } => {
            match state.add_group_member(&group_id, member_hash.clone()) {
                Ok(()) => {
                    // Notify all group members
                    if let Some(group) = state.get_group(&group_id) {
                        for member in &group.members {
                            if let Some(client) = state.clients.get(member) {
                                let _ = client.sender.send(encode_server_msg(&ServerMessage::GroupMemberAdded {
                                    group_id: group_id.clone(),
                                    member_hash: member_hash.clone(),
                                }));
                            }
                        }
                    }
                }
                Err(_e) => {
                    send_error(state, sender_hex, 400, "failed to add member");
                }
            }
        }

        // ----------------------------------------------------------------
        // GroupRemoveMember – remove member from group.
        // ----------------------------------------------------------------
        ClientMessage::GroupRemoveMember { group_id, member_hash } => {
            match state.remove_group_member(&group_id, &member_hash) {
                Ok(()) => {
                    if let Some(group) = state.get_group(&group_id) {
                        for member in &group.members {
                            if let Some(client) = state.clients.get(member) {
                                let _ = client.sender.send(encode_server_msg(&ServerMessage::GroupMemberRemoved {
                                    group_id: group_id.clone(),
                                    member_hash: member_hash.clone(),
                                }));
                            }
                        }
                    }
                }
                Err(_e) => {
                    send_error(state, sender_hex, 400, "failed to remove member");
                }
            }
        }

        // ----------------------------------------------------------------
        // GroupInfo – get group info.
        // ----------------------------------------------------------------
        ClientMessage::GroupInfo { group_id } => {
            match state.get_group(&group_id) {
                Some(group) => {
                    if let Some(client) = state.clients.get(sender_hex) {
                        let _ = client.sender.send(encode_server_msg(&ServerMessage::GroupInfo {
                            group_id: group.id,
                            name: group.name,
                            members: group.members,
                            owner: group.owner,
                        }));
                    }
                }
                None => {
                    send_error(state, sender_hex, 404, "group not found");
                }
            }
        }

        // ----------------------------------------------------------------
        // GroupLeave – leave a group.
        // ----------------------------------------------------------------
        ClientMessage::GroupLeave { group_id } => {
            match state.remove_group_member(&group_id, sender_hex) {
                Ok(()) => {
                    debug!(group_id = %group_id, user = %sender_hex, "left group");
                }
                Err(_e) => {
                    send_error(state, sender_hex, 400, "failed to leave group");
                }
            }
        }

        // ----------------------------------------------------------------
        // AckDelivered – acknowledge delivery.
        // ----------------------------------------------------------------
        ClientMessage::AckDelivered { message_id } => {
            state.record_delivery(message_id);
        }

        // ----------------------------------------------------------------
        // Identify after handshake – ignore (already authenticated).
        // ----------------------------------------------------------------
        ClientMessage::Identify { .. } => {
            send_error(state, sender_hex, 400, "already identified");
        }
    }
}

// ---------------------------------------------------------------------------
// Tiny utilities
// ---------------------------------------------------------------------------

fn send_error(state: &Arc<AppState>, recipient_hex: &str, code: u16, message: &'static str) {
    if let Some(client) = state.clients.get(recipient_hex) {
        let _ = client.sender.send(encode_server_msg(&ServerMessage::Error {
            code,
            message: message.into(),
        }));
    }
}

fn decode_recipient_hash_str(hex_str: &str) -> Result<String, RelayError> {
    let bytes = hex::decode(hex_str).map_err(|_| RelayError::InvalidHex)?;
    if bytes.len() != 32 {
        return Err(RelayError::InvalidRecipientHash(
            "must be 32 bytes (64 hex chars)".into(),
        ));
    }
    Ok(hex::encode(bytes))
}

fn received_at_from_instant(instant: Instant) -> u64 {
    // Convert a past Instant to an approximate Unix timestamp.
    let elapsed = instant.elapsed();
    unix_now().saturating_sub(elapsed.as_secs())
}

/// Extension trait used to compute approximate Unix timestamp from Instant elapsed.
trait NegFromUnix {
    fn checked_neg_from_unix(self) -> Option<u64>;
}

impl NegFromUnix for u64 {
    fn checked_neg_from_unix(self) -> Option<u64> {
        unix_now().checked_sub(self)
    }
}
