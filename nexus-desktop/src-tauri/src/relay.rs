use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

/// Messages from the client to the relay
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Identify {
        recipient_hash: String,
        challenge_response: String,
    },
    Send {
        recipient: String,
        sealed_content: String,
        ttl: u64,
    },
    FetchOffline,
    Ping {
        nonce: String,
    },
}

/// Messages from the relay to the client
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Challenge {
        nonce: String,
    },
    Deliver {
        sealed_content: String,
        received_at: u64,
    },
    OfflineMessages {
        messages: Vec<OfflineMessage>,
    },
    Delivered {
        id: String,
    },
    Error {
        code: u16,
        message: String,
    },
    Pong {
        nonce: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OfflineMessage {
    pub sealed_content: String,
    pub received_at: u64,
}

/// WebSocket relay client
pub struct RelayClient {
    url: String,
    sender: Option<mpsc::UnboundedSender<ClientMessage>>,
}

impl RelayClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            sender: None,
        }
    }

    /// Connect to the relay server
    pub async fn connect(
        &mut self,
        recipient_hash: String,
        message_handler: impl Fn(ServerMessage) + Send + 'static,
    ) -> Result<(), String> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Wait for challenge
        if let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
                let server_msg: ServerMessage = serde_json::from_str(&text)
                    .map_err(|e| format!("Failed to parse challenge: {}", e))?;

                if let ServerMessage::Challenge { nonce } = server_msg {
                    // Send identify message
                    let identify = ClientMessage::Identify {
                        recipient_hash: recipient_hash.clone(),
                        challenge_response: format!("signed_{}", nonce),
                    };
                    let identify_json = serde_json::to_string(&identify)
                        .map_err(|e| e.to_string())?;
                    ws_sender.send(Message::Text(identify_json))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        // Create channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<ClientMessage>();
        self.sender = Some(tx);

        // Spawn sender task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });

        // Spawn receiver task
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        message_handler(server_msg);
                    }
                }
            }
        });

        Ok(())
    }

    /// Send a message through the relay
    pub fn send_message(
        &self,
        recipient: String,
        sealed_content: String,
        ttl: u64,
    ) -> Result<(), String> {
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(ClientMessage::Send {
                recipient,
                sealed_content,
                ttl,
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Request offline messages
    pub fn fetch_offline(&self) -> Result<(), String> {
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender.send(ClientMessage::FetchOffline).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Send a ping
    pub fn ping(&self, nonce: String) -> Result<(), String> {
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender.send(ClientMessage::Ping { nonce }).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Disconnect from the relay
    pub fn disconnect(&mut self) {
        self.sender = None;
    }
}
