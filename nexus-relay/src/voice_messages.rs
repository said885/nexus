#![allow(missing_docs, dead_code)]

// Voice messages module
// nexus-relay/src/voice_messages.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum AudioCodec {
    Opus,
    AAC,
    FLAC,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct VoiceMessage {
    pub id: String,
    pub message_id: String,
    pub sender_id: String,
    pub duration_ms: u32,
    pub codec: AudioCodec,
    pub waveform_data: Vec<f32>,
    pub file_size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct VoiceTranscription {
    pub voice_message_id: String,
    pub language: String,
    pub confidence: f32,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

pub(crate) struct VoiceMessageManager {
    messages: HashMap<String, VoiceMessage>,
    transcriptions: HashMap<String, VoiceTranscription>,
    max_duration_ms: u32,
}

impl VoiceMessageManager {
    pub(crate) fn new() -> Self {
        VoiceMessageManager {
            messages: HashMap::new(),
            transcriptions: HashMap::new(),
            max_duration_ms: 300000, // 5 minutes max
        }
    }

    pub(crate) fn create_voice_message(
        &mut self,
        message_id: &str,
        sender_id: &str,
        duration_ms: u32,
        codec: AudioCodec,
        waveform_data: Vec<f32>,
        file_size_bytes: u64,
    ) -> Result<VoiceMessage, String> {
        if duration_ms > self.max_duration_ms {
            return Err(format!(
                "Voice message exceeds max duration of {}ms",
                self.max_duration_ms
            ));
        }

        let voice_message = VoiceMessage {
            id: format!("voice_{}", uuid::Uuid::new_v4()),
            message_id: message_id.to_string(),
            sender_id: sender_id.to_string(),
            duration_ms,
            codec,
            waveform_data,
            file_size_bytes,
            created_at: Utc::now(),
        };

        self.messages
            .insert(voice_message.id.clone(), voice_message.clone());
        Ok(voice_message)
    }

    pub(crate) fn add_transcription(
        &mut self,
        voice_message_id: &str,
        language: &str,
        confidence: f32,
        text: &str,
    ) -> Result<VoiceTranscription, String> {
        if confidence < 0.0 || confidence > 1.0 {
            return Err("Invalid confidence value".to_string());
        }

        let transcription = VoiceTranscription {
            voice_message_id: voice_message_id.to_string(),
            language: language.to_string(),
            confidence,
            text: text.to_string(),
            created_at: Utc::now(),
        };

        self.transcriptions.insert(
            voice_message_id.to_string(),
            transcription.clone(),
        );
        Ok(transcription)
    }

    pub(crate) fn get_voice_message(&self, id: &str) -> Option<&VoiceMessage> {
        self.messages.get(id)
    }

    pub(crate) fn get_transcription(&self, voice_message_id: &str) -> Option<&VoiceTranscription> {
        self.transcriptions.get(voice_message_id)
    }

    pub(crate) fn get_waveform(&self, voice_message_id: &str) -> Option<Vec<f32>> {
        self.messages
            .get(voice_message_id)
            .map(|vm| vm.waveform_data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_voice_message() {
        let mut manager = VoiceMessageManager::new();
        let waveform = vec![0.1, 0.2, 0.3, 0.4];
        let result = manager.create_voice_message(
            "msg_1",
            "user_1",
            5000,
            AudioCodec::Opus,
            waveform.clone(),
            15000,
        );
        assert!(result.is_ok());
        let vm = result.unwrap();
        assert_eq!(vm.sender_id, "user_1");
        assert_eq!(vm.duration_ms, 5000);
    }

    #[test]
    fn test_voice_message_too_long() {
        let mut manager = VoiceMessageManager::new();
        let waveform = vec![0.1; 1000];
        let result = manager.create_voice_message(
            "msg_1",
            "user_1",
            400000, // Exceeds max 300000
            AudioCodec::Opus,
            waveform,
            50000,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_transcription() {
        let mut manager = VoiceMessageManager::new();
        let waveform = vec![0.1, 0.2];
        let vm = manager
            .create_voice_message("msg_1", "user_1", 5000, AudioCodec::Opus, waveform, 10000)
            .unwrap();

        let result = manager.add_transcription(&vm.id, "en", 0.95, "Hello world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().text, "Hello world");
    }

    #[test]
    fn test_get_waveform() {
        let mut manager = VoiceMessageManager::new();
        let waveform = vec![0.1, 0.2, 0.3];
        let vm = manager
            .create_voice_message("msg_1", "user_1", 5000, AudioCodec::Opus, waveform.clone(), 10000)
            .unwrap();

        let retrieved = manager.get_waveform(&vm.id);
        assert_eq!(retrieved, Some(waveform));
    }
}
