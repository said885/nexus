use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Conversation metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub participant_hash: String,
    pub display_name: String,
    pub last_message: Option<String>,
    pub last_message_time: Option<u64>,
    pub unread_count: u32,
    pub created_at: u64,
}

/// Stored message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub conversation_id: String,
    pub content: String,
    pub is_outgoing: bool,
    pub timestamp: u64,
    pub delivered: bool,
    pub read: bool,
}

/// Local storage manager
pub struct LocalStorage {
    data_dir: PathBuf,
    conversations: HashMap<String, Conversation>,
    messages: HashMap<String, Vec<StoredMessage>>,
}

impl LocalStorage {
    pub fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not find data directory")?
            .join("nexus-messenger");

        fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;

        let mut storage = Self {
            data_dir,
            conversations: HashMap::new(),
            messages: HashMap::new(),
        };

        storage.load()?;
        Ok(storage)
    }

    /// Load data from disk
    fn load(&mut self) -> Result<(), String> {
        let conv_file = self.data_dir.join("conversations.json");
        if conv_file.exists() {
            let data = fs::read_to_string(&conv_file).map_err(|e| e.to_string())?;
            self.conversations = serde_json::from_str(&data).unwrap_or_default();
        }

        let msg_file = self.data_dir.join("messages.json");
        if msg_file.exists() {
            let data = fs::read_to_string(&msg_file).map_err(|e| e.to_string())?;
            self.messages = serde_json::from_str(&data).unwrap_or_default();
        }

        Ok(())
    }

    /// Save data to disk
    fn save(&self) -> Result<(), String> {
        let conv_file = self.data_dir.join("conversations.json");
        let data = serde_json::to_string_pretty(&self.conversations).map_err(|e| e.to_string())?;
        fs::write(&conv_file, data).map_err(|e| e.to_string())?;

        let msg_file = self.data_dir.join("messages.json");
        let data = serde_json::to_string_pretty(&self.messages).map_err(|e| e.to_string())?;
        fs::write(&msg_file, data).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Get all conversations
    pub fn get_conversations(&self) -> Vec<Conversation> {
        let mut convs: Vec<_> = self.conversations.values().cloned().collect();
        convs.sort_by(|a, b| {
            b.last_message_time
                .unwrap_or(0)
                .cmp(&a.last_message_time.unwrap_or(0))
        });
        convs
    }

    /// Create a new conversation
    pub fn create_conversation(
        &mut self,
        participant_hash: String,
        display_name: String,
    ) -> Result<Conversation, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let conv = Conversation {
            id: id.clone(),
            participant_hash,
            display_name,
            last_message: None,
            last_message_time: None,
            unread_count: 0,
            created_at: now,
        };

        self.conversations.insert(id.clone(), conv.clone());
        self.messages.insert(id, Vec::new());
        self.save()?;

        Ok(conv)
    }

    /// Get messages for a conversation
    pub fn get_messages(&self, conversation_id: &str) -> Vec<StoredMessage> {
        self.messages
            .get(conversation_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Save a message
    pub fn save_message(&mut self, message: StoredMessage) -> Result<(), String> {
        let conv_id = message.conversation_id.clone();
        let msgs = self.messages.entry(conv_id.clone()).or_default();
        msgs.push(message.clone());

        // Update conversation
        if let Some(conv) = self.conversations.get_mut(&conv_id) {
            conv.last_message = Some(message.content.clone());
            conv.last_message_time = Some(message.timestamp);
            if !message.is_outgoing && !message.read {
                conv.unread_count += 1;
            }
        }

        self.save()?;
        Ok(())
    }

    /// Mark messages as read
    pub fn mark_as_read(&mut self, conversation_id: &str) -> Result<(), String> {
        if let Some(conv) = self.conversations.get_mut(conversation_id) {
            conv.unread_count = 0;
        }
        if let Some(msgs) = self.messages.get_mut(conversation_id) {
            for msg in msgs.iter_mut() {
                if !msg.is_outgoing {
                    msg.read = true;
                }
            }
        }
        self.save()?;
        Ok(())
    }

    /// Delete a conversation and its messages
    pub fn delete_conversation(&mut self, conversation_id: &str) -> Result<(), String> {
        self.conversations.remove(conversation_id);
        self.messages.remove(conversation_id);
        self.save()?;
        Ok(())
    }
}
