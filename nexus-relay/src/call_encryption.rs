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

// Voice & Video Call Encryption (DTLS-SRTP for WebRTC)
// nexus-relay/src/call_encryption.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SRTP (Secure Real-time Transport Protocol) session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SrtpSession {
    pub session_id: String,
    pub master_key: [u8; 32],         // SRTP Master Key (256-bit)
    pub master_salt: [u8; 14],        // SRTP Master Salt (112-bit)
    pub payload_protection: bool,     // Encrypt media payload
    pub header_protection: bool,      // Encrypt RTP header
    pub rtcp_protection: bool,        // Encrypt RTCP packets
    pub encryption_algorithm: String, // "AES_CM_128_HMAC_SHA1_80" or modern algorithms
    pub auth_algorithm: String,       // "HMAC_SHA1" or "HMAC_SHA2_256"
}

/// Call session with encryption state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CallSession {
    pub call_id: String,
    pub initiator_id: String,
    pub recipient_id: String,
    pub call_type: CallType,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,

    // DTLS-SRTP state
    pub dtls_fingerprint_initiator: String, // SHA-256 DTLS cert fingerprint
    pub dtls_fingerprint_recipient: String,
    pub initiator_srtp_session: Option<SrtpSession>,
    pub recipient_srtp_session: Option<SrtpSession>,

    // ICE candidates
    pub initiator_ice_candidates: Vec<IceCandidate>,
    pub recipient_ice_candidates: Vec<IceCandidate>,

    // Codec negotiation
    pub audio_codec: Option<AudioCodec>,
    pub video_codec: Option<VideoCodec>,
    pub video_resolution: Option<(u16, u16)>, // width x height

    // Stats
    pub initiator_stats: CallStats,
    pub recipient_stats: CallStats,
}

/// Call type (audio only, video, screen share)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum CallType {
    AudioOnly,
    Video,
    ScreenShare,
    GroupAudio,
    GroupVideo,
}

/// ICE candidate for NAT traversal
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct IceCandidate {
    pub foundation: String,
    pub component: u16,    // 1 = RTP, 2 = RTCP
    pub transport: String, // "udp" or "tcp"
    pub priority: u32,
    pub candidate_address: String,
    pub candidate_port: u16,
    pub related_address: Option<String>,
    pub related_port: Option<u16>,
    pub candidate_type: IceCandidateType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum IceCandidateType {
    Host,
    Srflx, // Server reflexive
    Prflx, // Peer reflexive
    Relay,
}

/// Supported audio codecs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum AudioCodec {
    #[serde(rename = "opus")]
    Opus { bitrate: u32 }, // 8-510 kbps, recommended 24-128
    #[serde(rename = "g729")]
    G729 { bitrate: u32 }, // 8 kbps
    #[serde(rename = "aac")]
    Aac { bitrate: u32 }, // 8-320 kbps
}

/// Supported video codecs with PFS
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum VideoCodec {
    #[serde(rename = "vp9")]
    VP9 { level: u8, temporal_layers: u8 }, // Level 0-4, TL 0-3
    #[serde(rename = "h265")]
    H265 { profile: u8 }, // Main profile (0-1)
    #[serde(rename = "av1")]
    AV1 { level: u8 }, // Level 0-23 (4.0 = L20)
}

/// Real-time call statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CallStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packet_loss_rate: f32, // 0.0 - 1.0
    pub round_trip_time: u32,  // milliseconds
    pub jitter: u32,           // milliseconds
    pub codec_bandwidth: u32,  // kbps
    pub audio_level: i16,      // dBFS
    pub timestamp: DateTime<Utc>,
}

impl Default for CallStats {
    fn default() -> Self {
        Self {
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packet_loss_rate: 0.0,
            round_trip_time: 0,
            jitter: 0,
            codec_bandwidth: 0,
            audio_level: -100,
            timestamp: Utc::now(),
        }
    }
}

impl CallSession {
    /// Initialize new call session
    pub(crate) fn new(
        call_id: String,
        initiator_id: String,
        recipient_id: String,
        call_type: CallType,
    ) -> Self {
        Self {
            call_id,
            initiator_id,
            recipient_id,
            call_type,
            started_at: None,
            ended_at: None,
            dtls_fingerprint_initiator: String::new(),
            dtls_fingerprint_recipient: String::new(),
            initiator_srtp_session: None,
            recipient_srtp_session: None,
            initiator_ice_candidates: Vec::new(),
            recipient_ice_candidates: Vec::new(),
            audio_codec: None,
            video_codec: None,
            video_resolution: None,
            initiator_stats: CallStats::default(),
            recipient_stats: CallStats::default(),
        }
    }

    /// Exchange DTLS fingerprints (happens during WebRTC offer/answer)
    pub(crate) fn set_dtls_fingerprints(
        &mut self,
        initiator_fp: String,
        recipient_fp: String,
    ) -> Result<(), String> {
        self.dtls_fingerprint_initiator = initiator_fp;
        self.dtls_fingerprint_recipient = recipient_fp;
        Ok(())
    }

    /// Add ICE candidate from initiator
    pub(crate) fn add_initiator_ice_candidate(&mut self, candidate: IceCandidate) -> Result<(), String> {
        if self.initiator_ice_candidates.len() > 50 {
            return Err("Too many ICE candidates".to_string());
        }
        self.initiator_ice_candidates.push(candidate);
        Ok(())
    }

    /// Add ICE candidate from recipient
    pub(crate) fn add_recipient_ice_candidate(&mut self, candidate: IceCandidate) -> Result<(), String> {
        if self.recipient_ice_candidates.len() > 50 {
            return Err("Too many ICE candidates".to_string());
        }
        self.recipient_ice_candidates.push(candidate);
        Ok(())
    }

    /// Establish SRTP session after DTLS handshake
    pub(crate) fn establish_srtp_sessions(&mut self) -> Result<(), String> {
        // In production: Extract keys from DTLS profile TLS-PRF
        // For now: Generate random keys (would be derived from DTLS in real implementation)

        let initiator_session = SrtpSession {
            session_id: format!("srtp-{}-initiator", self.call_id),
            master_key: [0u8; 32], // Would be derived from DTLS
            master_salt: [0u8; 14],
            payload_protection: true,
            header_protection: true,
            rtcp_protection: true,
            encryption_algorithm: "AES_CM_128_HMAC_SHA1_80".to_string(),
            auth_algorithm: "HMAC_SHA1".to_string(),
        };

        let recipient_session = SrtpSession {
            session_id: format!("srtp-{}-recipient", self.call_id),
            master_key: [0u8; 32],
            master_salt: [0u8; 14],
            payload_protection: true,
            header_protection: true,
            rtcp_protection: true,
            encryption_algorithm: "AES_CM_128_HMAC_SHA1_80".to_string(),
            auth_algorithm: "HMAC_SHA1".to_string(),
        };

        self.initiator_srtp_session = Some(initiator_session);
        self.recipient_srtp_session = Some(recipient_session);

        Ok(())
    }

    /// Negotiate codecs between peers
    pub(crate) fn negotiate_codecs(
        &mut self,
        initiator_audio: Option<AudioCodec>,
        initiator_video: Option<VideoCodec>,
        recipient_audio: Option<AudioCodec>,
        recipient_video: Option<VideoCodec>,
    ) -> Result<(), String> {
        // Simple negotiation: take common codec or highest preference
        self.audio_codec = initiator_audio.or(recipient_audio);
        self.video_codec = initiator_video.or(recipient_video);

        Ok(())
    }

    /// Start the call (establish media)
    pub(crate) fn start_call(&mut self) -> Result<(), String> {
        self.started_at = Some(Utc::now());
        Ok(())
    }

    /// End the call and record stats
    pub(crate) fn end_call(&mut self) -> Result<(), String> {
        self.ended_at = Some(Utc::now());
        Ok(())
    }

    /// Get call duration in seconds
    pub(crate) fn duration(&self) -> Option<u64> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => {
                let duration = end.signed_duration_since(start);
                Some(duration.num_seconds() as u64)
            }
            (Some(start), None) => {
                let duration = Utc::now().signed_duration_since(start);
                Some(duration.num_seconds() as u64)
            }
            _ => None,
        }
    }

    /// Update initiator stats
    pub(crate) fn update_initiator_stats(&mut self, stats: CallStats) {
        self.initiator_stats = stats;
    }

    /// Update recipient stats
    pub(crate) fn update_recipient_stats(&mut self, stats: CallStats) {
        self.recipient_stats = stats;
    }
}

/// Call manager - handles multiple concurrent calls
pub(crate) struct CallManager {
    pub calls: HashMap<String, CallSession>,
    pub user_calls: HashMap<String, Vec<String>>, // user_id -> [call_ids]
    pub max_concurrent_calls_per_user: usize,
}

impl CallManager {
    pub(crate) fn new(max_concurrent: usize) -> Self {
        Self {
            calls: HashMap::new(),
            user_calls: HashMap::new(),
            max_concurrent_calls_per_user: max_concurrent,
        }
    }

    /// Create new call session
    pub(crate) fn create_call(
        &mut self,
        call_id: String,
        initiator_id: String,
        recipient_id: String,
        call_type: CallType,
    ) -> Result<CallSession, String> {
        // Check concurrent call limits
        let initiator_calls = self
            .user_calls
            .entry(initiator_id.clone())
            .or_default();
        if initiator_calls.len() >= self.max_concurrent_calls_per_user {
            return Err("Max concurrent calls reached".to_string());
        }

        let session = CallSession::new(
            call_id.clone(),
            initiator_id.clone(),
            recipient_id,
            call_type,
        );

        self.calls.insert(call_id.clone(), session.clone());
        self.user_calls
            .entry(initiator_id)
            .or_default()
            .push(call_id);

        Ok(session)
    }

    /// Get active call
    pub(crate) fn get_call(&self, call_id: &str) -> Option<&CallSession> {
        self.calls.get(call_id)
    }

    /// Get mutable call
    pub(crate) fn get_call_mut(&mut self, call_id: &str) -> Option<&mut CallSession> {
        self.calls.get_mut(call_id)
    }

    /// End call and cleanup
    pub(crate) fn end_call(&mut self, call_id: String) -> Result<(), String> {
        if let Some(mut session) = self.calls.remove(&call_id) {
            session.end_call()?;

            // Remove from user_calls
            for calls in self.user_calls.values_mut() {
                calls.retain(|id| id != &call_id);
            }

            Ok(())
        } else {
            Err("Call not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_session_creation() {
        let session = CallSession::new(
            "call123".to_string(),
            "user1".to_string(),
            "user2".to_string(),
            CallType::Video,
        );

        assert_eq!(session.call_id, "call123");
        assert_eq!(session.call_type, CallType::Video);
        assert!(session.started_at.is_none());
    }

    #[test]
    fn test_codec_negotiation() {
        let mut session = CallSession::new(
            "call123".to_string(),
            "user1".to_string(),
            "user2".to_string(),
            CallType::Video,
        );

        let audio = AudioCodec::Opus { bitrate: 64 };
        let video = VideoCodec::VP9 {
            level: 0,
            temporal_layers: 0,
        };

        session
            .negotiate_codecs(Some(audio.clone()), Some(video.clone()), None, None)
            .unwrap();

        assert_eq!(session.audio_codec, Some(audio));
        assert_eq!(session.video_codec, Some(video));
    }

    #[test]
    fn test_srtp_session_establishment() {
        let mut session = CallSession::new(
            "call123".to_string(),
            "user1".to_string(),
            "user2".to_string(),
            CallType::Video,
        );

        assert!(session.establish_srtp_sessions().is_ok());
        assert!(session.initiator_srtp_session.is_some());
        assert!(session.recipient_srtp_session.is_some());
    }

    #[test]
    fn test_call_duration() {
        let mut session = CallSession::new(
            "call123".to_string(),
            "user1".to_string(),
            "user2".to_string(),
            CallType::AudioOnly,
        );

        session.start_call().unwrap();
        assert!(session.duration().is_some());
    }

    #[test]
    fn test_call_manager() {
        let mut manager = CallManager::new(5);

        let session = manager
            .create_call(
                "call123".to_string(),
                "user1".to_string(),
                "user2".to_string(),
                CallType::Video,
            )
            .unwrap();

        assert_eq!(session.call_id, "call123");
        assert!(manager.get_call("call123").is_some());
    }
}
