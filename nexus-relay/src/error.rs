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

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    // Rate limiting
    #[error("rate limit exceeded")]
    RateLimitExceeded,

    #[error("connection limit exceeded for IP")]
    ConnectionLimitExceeded,

    // Validation errors
    #[error("invalid recipient hash: {0}")]
    InvalidRecipientHash(String),

    #[error("invalid hex encoding")]
    InvalidHex,

    #[error("invalid base64 encoding")]
    InvalidBase64,

    #[error("invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("payload too large (max {max_bytes} bytes)")]
    PayloadTooLarge { max_bytes: usize },

    #[error("TTL out of range (max 7 days)")]
    TtlOutOfRange,

    #[error("invalid group name: {0}")]
    InvalidGroupName(String),

    #[error("invalid user status: {0}")]
    InvalidUserStatus(String),

    // Recipient errors
    #[error("recipient not found")]
    RecipientNotFound,

    #[error("recipient offline queue full")]
    QueueFull,

    // Identity errors
    #[error("identity already registered")]
    AlreadyRegistered,

    #[error("invalid prekey bundle")]
    InvalidPreKeyBundle,

    #[error("prekey bundle not found")]
    PreKeyBundleNotFound,

    // Authentication errors
    #[error("not authenticated")]
    NotAuthenticated,

    #[error("authentication failed")]
    AuthenticationFailed,

    #[error("challenge expired")]
    ChallengeExpired,

    // Group errors
    #[error("group not found")]
    GroupNotFound,

    #[error("not a group member")]
    NotGroupMember,

    #[error("group is full (max {max_members} members)")]
    GroupFull { max_members: usize },

    #[error("already a group member")]
    AlreadyGroupMember,

    #[error("cannot remove group owner")]
    CannotRemoveOwner,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    // Call errors
    #[error("call not found")]
    CallNotFound,

    #[error("call already ended")]
    CallAlreadyEnded,

    #[error("user already in a call")]
    UserAlreadyInCall,

    // Internal errors
    #[error("internal error: {0}")]
    Internal(String),

    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),
}

impl RelayError {
    pub fn status_code(&self) -> u16 {
        match self {
            // Rate limiting
            RelayError::RateLimitExceeded => 429,
            RelayError::ConnectionLimitExceeded => 429,

            // Validation
            RelayError::InvalidRecipientHash(_) => 400,
            RelayError::InvalidHex => 400,
            RelayError::InvalidBase64 => 400,
            RelayError::InvalidMessageFormat(_) => 400,
            RelayError::PayloadTooLarge { .. } => 413,
            RelayError::TtlOutOfRange => 400,
            RelayError::InvalidGroupName(_) => 400,
            RelayError::InvalidUserStatus(_) => 400,

            // Recipient
            RelayError::RecipientNotFound => 404,
            RelayError::QueueFull => 507,

            // Identity
            RelayError::AlreadyRegistered => 409,
            RelayError::InvalidPreKeyBundle => 400,
            RelayError::PreKeyBundleNotFound => 404,

            // Auth
            RelayError::NotAuthenticated => 401,
            RelayError::AuthenticationFailed => 401,
            RelayError::ChallengeExpired => 401,

            // Groups
            RelayError::GroupNotFound => 404,
            RelayError::NotGroupMember => 403,
            RelayError::GroupFull { .. } => 400,
            RelayError::AlreadyGroupMember => 409,
            RelayError::CannotRemoveOwner => 400,
            RelayError::InsufficientPermissions => 403,

            // Calls
            RelayError::CallNotFound => 404,
            RelayError::CallAlreadyEnded => 400,
            RelayError::UserAlreadyInCall => 409,

            // Internal
            RelayError::Internal(_) => 500,
            RelayError::DatabaseError(_) => 500,
            RelayError::SerializationError(_) => 500,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            RelayError::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            RelayError::ConnectionLimitExceeded => "CONNECTION_LIMIT_EXCEEDED",
            RelayError::InvalidRecipientHash(_) => "INVALID_RECIPIENT_HASH",
            RelayError::InvalidHex => "INVALID_HEX",
            RelayError::InvalidBase64 => "INVALID_BASE64",
            RelayError::InvalidMessageFormat(_) => "INVALID_MESSAGE_FORMAT",
            RelayError::PayloadTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            RelayError::TtlOutOfRange => "TTL_OUT_OF_RANGE",
            RelayError::InvalidGroupName(_) => "INVALID_GROUP_NAME",
            RelayError::InvalidUserStatus(_) => "INVALID_USER_STATUS",
            RelayError::RecipientNotFound => "RECIPIENT_NOT_FOUND",
            RelayError::QueueFull => "QUEUE_FULL",
            RelayError::AlreadyRegistered => "ALREADY_REGISTERED",
            RelayError::InvalidPreKeyBundle => "INVALID_PREKEY_BUNDLE",
            RelayError::PreKeyBundleNotFound => "PREKEY_BUNDLE_NOT_FOUND",
            RelayError::NotAuthenticated => "NOT_AUTHENTICATED",
            RelayError::AuthenticationFailed => "AUTHENTICATION_FAILED",
            RelayError::ChallengeExpired => "CHALLENGE_EXPIRED",
            RelayError::GroupNotFound => "GROUP_NOT_FOUND",
            RelayError::NotGroupMember => "NOT_GROUP_MEMBER",
            RelayError::GroupFull { .. } => "GROUP_FULL",
            RelayError::AlreadyGroupMember => "ALREADY_GROUP_MEMBER",
            RelayError::CannotRemoveOwner => "CANNOT_REMOVE_OWNER",
            RelayError::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",
            RelayError::CallNotFound => "CALL_NOT_FOUND",
            RelayError::CallAlreadyEnded => "CALL_ALREADY_ENDED",
            RelayError::UserAlreadyInCall => "USER_ALREADY_IN_CALL",
            RelayError::Internal(_) => "INTERNAL_ERROR",
            RelayError::DatabaseError(_) => "DATABASE_ERROR",
            RelayError::SerializationError(_) => "SERIALIZATION_ERROR",
        }
    }
}

impl IntoResponse for RelayError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = Json(json!({
            "error": {
                "code": self.error_code(),
                "status": self.status_code(),
                "message": self.to_string(),
            }
        }));

        (status, body).into_response()
    }
}

// Convenience conversions
impl From<serde_json::Error> for RelayError {
    fn from(err: serde_json::Error) -> Self {
        RelayError::SerializationError(err.to_string())
    }
}

impl From<hex::FromHexError> for RelayError {
    fn from(_: hex::FromHexError) -> Self {
        RelayError::InvalidHex
    }
}

impl From<base64::DecodeError> for RelayError {
    fn from(_: base64::DecodeError) -> Self {
        RelayError::InvalidBase64
    }
}
