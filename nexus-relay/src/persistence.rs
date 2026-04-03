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

//! PostgreSQL Persistence Layer for NEXUS Relay Server
//! 
//! Provides persistent storage for all server state including:
//! - User identities and prekeys
//! - Offline messages
//! - Group chats
//! - Call sessions
//! - Security alerts
//! - Audit logs

use sqlx::{PgPool, postgres::{PgPoolOptions, PgRow}, Row, Error as SqlxError};
use std::time::Duration;
use chrono::{DateTime, Utc};
use tracing::info;

/// Database configuration
#[derive(Debug, Clone)]
pub(crate) struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://nexus:nexus@localhost:5432/nexus".to_string()),
            max_connections: 100,
            min_connections: 10,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

/// Database connection pool wrapper
pub(crate) struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub(crate) async fn new(config: DatabaseConfig) -> Result<Self, SqlxError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.url)
            .await?;

        info!("Database connection pool established");
        
        Ok(Self { pool })
    }

    /// Run database migrations
    pub(crate) async fn migrate(&self) -> Result<(), SqlxError> {
        info!("Running database migrations...");
        
        // Create tables if not exist
        sqlx::query(r#"
            CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
            CREATE EXTENSION IF NOT EXISTS "pgcrypto";
            
            -- Users table
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                recipient_hash VARCHAR(64) UNIQUE NOT NULL,
                identity_key BYTEA NOT NULL,
                signed_prekey BYTEA NOT NULL,
                signed_prekey_signature BYTEA NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                last_seen TIMESTAMPTZ,
                status VARCHAR(20) DEFAULT 'offline',
                status_message TEXT,
                is_active BOOLEAN DEFAULT TRUE,
                dp_request_count BIGINT DEFAULT 0,
                dp_last_bucket TIMESTAMPTZ
            );
            
            -- One-time prekeys
            CREATE TABLE IF NOT EXISTS one_time_prekeys (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                prekey_index INTEGER NOT NULL,
                prekey_data BYTEA NOT NULL,
                is_used BOOLEAN DEFAULT FALSE,
                used_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(user_id, prekey_index)
            );
            CREATE INDEX IF NOT EXISTS idx_otp_user ON one_time_prekeys(user_id) WHERE is_used = FALSE;
            
            -- Offline messages
            CREATE TABLE IF NOT EXISTS offline_messages (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                recipient_hash VARCHAR(64) NOT NULL,
                sealed_content BYTEA NOT NULL,
                sender_hash VARCHAR(64),
                received_at TIMESTAMPTZ DEFAULT NOW(),
                expires_at TIMESTAMPTZ NOT NULL,
                priority INTEGER DEFAULT 0,
                is_delivered BOOLEAN DEFAULT FALSE,
                delivered_at TIMESTAMPTZ,
                dp_size_bucket INTEGER,
                dp_time_bucket TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS idx_om_recipient ON offline_messages(recipient_hash) WHERE is_delivered = FALSE;
            CREATE INDEX IF NOT EXISTS idx_om_expires ON offline_messages(expires_at) WHERE is_delivered = FALSE;
            
            -- Groups
            CREATE TABLE IF NOT EXISTS groups (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                name VARCHAR(100) NOT NULL,
                description TEXT,
                owner_hash VARCHAR(64) NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW(),
                epoch BIGINT DEFAULT 0,
                max_members INTEGER DEFAULT 256,
                is_public BOOLEAN DEFAULT FALSE
            );
            
            -- Group members
            CREATE TABLE IF NOT EXISTS group_members (
                group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                member_hash VARCHAR(64) NOT NULL,
                is_admin BOOLEAN DEFAULT FALSE,
                joined_at TIMESTAMPTZ DEFAULT NOW(),
                PRIMARY KEY (group_id, member_hash)
            );
            CREATE INDEX IF NOT EXISTS idx_gm_member ON group_members(member_hash);
            
            -- Call sessions
            CREATE TABLE IF NOT EXISTS call_sessions (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                initiator_hash VARCHAR(64) NOT NULL,
                recipient_hash VARCHAR(64) NOT NULL,
                call_type VARCHAR(10) NOT NULL,
                status VARCHAR(20) DEFAULT 'ringing',
                started_at TIMESTAMPTZ DEFAULT NOW(),
                answered_at TIMESTAMPTZ,
                ended_at TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS idx_calls_initiator ON call_sessions(initiator_hash);
            CREATE INDEX IF NOT EXISTS idx_calls_active ON call_sessions(status) WHERE status IN ('ringing', 'active');
            
            -- Delivery receipts
            CREATE TABLE IF NOT EXISTS delivery_receipts (
                message_id UUID PRIMARY KEY,
                recipient_hash VARCHAR(64) NOT NULL,
                delivered_at TIMESTAMPTZ DEFAULT NOW()
            );
            
            -- Security alerts
            CREATE TABLE IF NOT EXISTS security_alerts (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                severity VARCHAR(20) NOT NULL,
                alert_type VARCHAR(50) NOT NULL,
                description TEXT NOT NULL,
                user_hash VARCHAR(64),
                ip_address INET,
                anomaly_score FLOAT,
                features JSONB,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                resolved_at TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS idx_sa_severity ON security_alerts(severity, created_at DESC);
            
            -- Rate limits
            CREATE TABLE IF NOT EXISTS rate_limits (
                ip_address INET PRIMARY KEY,
                request_count INTEGER DEFAULT 0,
                window_start TIMESTAMPTZ DEFAULT NOW(),
                blocked_until TIMESTAMPTZ
            );
            
            -- Audit log
            CREATE TABLE IF NOT EXISTS audit_log (
                id BIGSERIAL PRIMARY KEY,
                event_type VARCHAR(50) NOT NULL,
                user_hash VARCHAR(64),
                ip_address INET,
                details JSONB,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_al_user ON audit_log(user_hash, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_al_time ON audit_log(created_at DESC);
        "#)
        .execute(&self.pool)
        .await?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Get pool reference
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Health check
    pub(crate) async fn health_check(&self) -> Result<bool, SqlxError> {
        let result: i32 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(result == 1)
    }
}

// ============================================================================
// USER OPERATIONS
// ============================================================================

/// User repository
pub(crate) struct UserRepository<'a> {
    db: &'a Database,
}

impl<'a> UserRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Register a new user
    pub(crate) async fn register(
        &self,
        recipient_hash: &str,
        identity_key: &[u8],
        signed_prekey: &[u8],
        signed_prekey_signature: &[u8],
    ) -> Result<(), SqlxError> {
        sqlx::query(r#"
            INSERT INTO users (recipient_hash, identity_key, signed_prekey, signed_prekey_signature)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (recipient_hash) DO NOTHING
        "#)
        .bind(recipient_hash)
        .bind(identity_key)
        .bind(signed_prekey)
        .bind(signed_prekey_signature)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Get user by recipient hash
    pub(crate) async fn get_by_hash(&self, recipient_hash: &str) -> Result<Option<User>, SqlxError> {
        let row = sqlx::query(r#"
            SELECT id, recipient_hash, identity_key, signed_prekey, signed_prekey_signature,
                   created_at, last_seen, status, is_active
            FROM users
            WHERE recipient_hash = $1 AND is_active = TRUE
        "#)
        .bind(recipient_hash)
        .fetch_optional(&self.db.pool)
        .await?;

        Ok(row.map(User::from_row))
    }

    /// Update user status
    pub(crate) async fn update_status(
        &self,
        recipient_hash: &str,
        status: &str,
        status_message: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(r#"
            UPDATE users 
            SET status = $2, status_message = $3, last_seen = NOW(), updated_at = NOW()
            WHERE recipient_hash = $1
        "#)
        .bind(recipient_hash)
        .bind(status)
        .bind(status_message)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Update last seen timestamp
    pub(crate) async fn touch(&self, recipient_hash: &str) -> Result<(), SqlxError> {
        sqlx::query(r#"
            UPDATE users SET last_seen = NOW() WHERE recipient_hash = $1
        "#)
        .bind(recipient_hash)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Get online users count
    pub(crate) async fn online_count(&self) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(r#"
            SELECT COUNT(*) FROM users 
            WHERE status = 'online' AND last_seen > NOW() - INTERVAL '5 minutes'
        "#)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(count)
    }
}

/// User model
#[derive(Debug, Clone)]
pub(crate) struct User {
    pub id: uuid::Uuid,
    pub recipient_hash: String,
    pub identity_key: Vec<u8>,
    pub signed_prekey: Vec<u8>,
    pub signed_prekey_signature: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
    pub status: String,
    pub is_active: bool,
}

impl User {
    fn from_row(row: PgRow) -> Self {
        Self {
            id: row.get("id"),
            recipient_hash: row.get("recipient_hash"),
            identity_key: row.get("identity_key"),
            signed_prekey: row.get("signed_prekey"),
            signed_prekey_signature: row.get("signed_prekey_signature"),
            created_at: row.get("created_at"),
            last_seen: row.get("last_seen"),
            status: row.get("status"),
            is_active: row.get("is_active"),
        }
    }
}

// ============================================================================
// PREKEY OPERATIONS
// ============================================================================

/// Prekey repository
pub(crate) struct PrekeyRepository<'a> {
    db: &'a Database,
}

impl<'a> PrekeyRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Store one-time prekeys
    pub(crate) async fn store_prekeys(
        &self,
        user_id: uuid::Uuid,
        prekeys: &[(i32, Vec<u8>)],
    ) -> Result<(), SqlxError> {
        let mut tx = self.db.pool.begin().await?;

        for (index, data) in prekeys {
            sqlx::query(r#"
                INSERT INTO one_time_prekeys (user_id, prekey_index, prekey_data)
                VALUES ($1, $2, $3)
                ON CONFLICT (user_id, prekey_index) DO NOTHING
            "#)
            .bind(user_id)
            .bind(index)
            .bind(data)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Fetch and consume a one-time prekey
    pub(crate) async fn fetch_and_consume(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<(i32, Vec<u8>)>, SqlxError> {
        let mut tx = self.db.pool.begin().await?;

        // Get oldest unused prekey
        let row = sqlx::query(r#"
            SELECT id, prekey_index, prekey_data
            FROM one_time_prekeys
            WHERE user_id = $1 AND is_used = FALSE
            ORDER BY prekey_index ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        "#)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            let id: uuid::Uuid = row.get("id");
            let index: i32 = row.get("prekey_index");
            let data: Vec<u8> = row.get("prekey_data");

            // Mark as used
            sqlx::query(r#"
                UPDATE one_time_prekeys 
                SET is_used = TRUE, used_at = NOW()
                WHERE id = $1
            "#)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            Ok(Some((index, data)))
        } else {
            tx.commit().await?;
            Ok(None)
        }
    }

    /// Get remaining prekey count
    pub(crate) async fn remaining_count(&self, user_id: uuid::Uuid) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(r#"
            SELECT COUNT(*) FROM one_time_prekeys
            WHERE user_id = $1 AND is_used = FALSE
        "#)
        .bind(user_id)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(count)
    }
}

// ============================================================================
// MESSAGE OPERATIONS
// ============================================================================

/// Message repository
pub(crate) struct MessageRepository<'a> {
    db: &'a Database,
}

impl<'a> MessageRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Queue an offline message
    pub(crate) async fn queue_message(
        &self,
        recipient_hash: &str,
        sealed_content: &[u8],
        sender_hash: Option<&str>,
        ttl_seconds: i64,
        priority: i32,
    ) -> Result<uuid::Uuid, SqlxError> {
        let id: uuid::Uuid = sqlx::query_scalar(r#"
            INSERT INTO offline_messages (recipient_hash, sealed_content, sender_hash, expires_at, priority)
            VALUES ($1, $2, $3, NOW() + ($4 || ' seconds')::INTERVAL, $5)
            RETURNING id
        "#)
        .bind(recipient_hash)
        .bind(sealed_content)
        .bind(sender_hash)
        .bind(ttl_seconds.to_string())
        .bind(priority)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(id)
    }

    /// Drain offline messages for a recipient
    pub(crate) async fn drain_messages(
        &self,
        recipient_hash: &str,
    ) -> Result<Vec<OfflineMessage>, SqlxError> {
        let mut tx = self.db.pool.begin().await?;

        // Get undelivered messages
        let rows = sqlx::query(r#"
            SELECT id, sealed_content, sender_hash, received_at, priority
            FROM offline_messages
            WHERE recipient_hash = $1 
              AND is_delivered = FALSE 
              AND expires_at > NOW()
            ORDER BY priority DESC, received_at ASC
            FOR UPDATE SKIP LOCKED
        "#)
        .bind(recipient_hash)
        .fetch_all(&mut *tx)
        .await?;

        let messages: Vec<OfflineMessage> = rows.iter().map(OfflineMessage::from_row).collect();

        // Mark as delivered
        if !messages.is_empty() {
            let ids: Vec<uuid::Uuid> = messages.iter().map(|m| m.id).collect();
            sqlx::query(r#"
                UPDATE offline_messages 
                SET is_delivered = TRUE, delivered_at = NOW()
                WHERE id = ANY($1)
            "#)
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(messages)
    }

    /// Get pending message count
    pub(crate) async fn pending_count(&self, recipient_hash: &str) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(r#"
            SELECT COUNT(*) FROM offline_messages
            WHERE recipient_hash = $1 AND is_delivered = FALSE AND expires_at > NOW()
        "#)
        .bind(recipient_hash)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(count)
    }

    /// Cleanup expired messages
    pub(crate) async fn cleanup_expired(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(r#"
            DELETE FROM offline_messages 
            WHERE expires_at < NOW() 
               OR (is_delivered = TRUE AND delivered_at < NOW() - INTERVAL '7 days')
        "#)
        .execute(&self.db.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get total queued messages
    pub(crate) async fn total_queued(&self) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(r#"
            SELECT COUNT(*) FROM offline_messages
            WHERE is_delivered = FALSE AND expires_at > NOW()
        "#)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(count)
    }
}

/// Offline message model
#[derive(Debug, Clone)]
pub(crate) struct OfflineMessage {
    pub id: uuid::Uuid,
    pub sealed_content: Vec<u8>,
    pub sender_hash: Option<String>,
    pub received_at: DateTime<Utc>,
    pub priority: i32,
}

impl OfflineMessage {
    fn from_row(row: &PgRow) -> Self {
        Self {
            id: row.get("id"),
            sealed_content: row.get("sealed_content"),
            sender_hash: row.get("sender_hash"),
            received_at: row.get("received_at"),
            priority: row.get("priority"),
        }
    }
}

// ============================================================================
// GROUP OPERATIONS
// ============================================================================

/// Group repository
pub(crate) struct GroupRepository<'a> {
    db: &'a Database,
}

impl<'a> GroupRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new group
    pub(crate) async fn create(
        &self,
        name: &str,
        owner_hash: &str,
        description: Option<&str>,
    ) -> Result<uuid::Uuid, SqlxError> {
        let mut tx = self.db.pool.begin().await?;

        // Create group
        let group_id: uuid::Uuid = sqlx::query_scalar(r#"
            INSERT INTO groups (name, owner_hash, description)
            VALUES ($1, $2, $3)
            RETURNING id
        "#)
        .bind(name)
        .bind(owner_hash)
        .bind(description)
        .fetch_one(&mut *tx)
        .await?;

        // Add owner as member
        sqlx::query(r#"
            INSERT INTO group_members (group_id, member_hash, is_admin)
            VALUES ($1, $2, TRUE)
        "#)
        .bind(group_id)
        .bind(owner_hash)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(group_id)
    }

    /// Get group by ID
    pub(crate) async fn get(&self, group_id: uuid::Uuid) -> Result<Option<Group>, SqlxError> {
        let row = sqlx::query(r#"
            SELECT id, name, description, owner_hash, created_at, epoch, max_members, is_public
            FROM groups
            WHERE id = $1
        "#)
        .bind(group_id)
        .fetch_optional(&self.db.pool)
        .await?;

        Ok(row.map(Group::from_row))
    }

    /// Add member to group
    pub(crate) async fn add_member(
        &self,
        group_id: uuid::Uuid,
        member_hash: &str,
    ) -> Result<(), SqlxError> {
        sqlx::query(r#"
            INSERT INTO group_members (group_id, member_hash)
            VALUES ($1, $2)
            ON CONFLICT (group_id, member_hash) DO NOTHING
        "#)
        .bind(group_id)
        .bind(member_hash)
        .execute(&self.db.pool)
        .await?;

        // Increment epoch
        sqlx::query(r#"
            UPDATE groups SET epoch = epoch + 1, updated_at = NOW()
            WHERE id = $1
        "#)
        .bind(group_id)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Remove member from group
    pub(crate) async fn remove_member(
        &self,
        group_id: uuid::Uuid,
        member_hash: &str,
    ) -> Result<bool, SqlxError> {
        // Don't remove owner
        let group = self.get(group_id).await?;
        if let Some(g) = group {
            if g.owner_hash == member_hash {
                return Ok(false);
            }
        }

        let result = sqlx::query(r#"
            DELETE FROM group_members
            WHERE group_id = $1 AND member_hash = $2
        "#)
        .bind(group_id)
        .bind(member_hash)
        .execute(&self.db.pool)
        .await?;

        if result.rows_affected() > 0 {
            sqlx::query(r#"
                UPDATE groups SET epoch = epoch + 1, updated_at = NOW()
                WHERE id = $1
            "#)
            .bind(group_id)
            .execute(&self.db.pool)
            .await?;
        }

        Ok(result.rows_affected() > 0)
    }

    /// Get group members
    pub(crate) async fn get_members(&self, group_id: uuid::Uuid) -> Result<Vec<GroupMember>, SqlxError> {
        let rows = sqlx::query(r#"
            SELECT member_hash, is_admin, joined_at
            FROM group_members
            WHERE group_id = $1
        "#)
        .bind(group_id)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(rows.iter().map(GroupMember::from_row).collect())
    }

    /// Check if user is member
    pub(crate) async fn is_member(
        &self,
        group_id: uuid::Uuid,
        member_hash: &str,
    ) -> Result<bool, SqlxError> {
        let exists: bool = sqlx::query_scalar(r#"
            SELECT EXISTS(
                SELECT 1 FROM group_members
                WHERE group_id = $1 AND member_hash = $2
            )
        "#)
        .bind(group_id)
        .bind(member_hash)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(exists)
    }

    /// Get total groups
    pub(crate) async fn total_groups(&self) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM groups")
            .fetch_one(&self.db.pool)
            .await?;
        Ok(count)
    }
}

/// Group model
#[derive(Debug, Clone)]
pub(crate) struct Group {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_hash: String,
    pub created_at: DateTime<Utc>,
    pub epoch: i64,
    pub max_members: i32,
    pub is_public: bool,
}

impl Group {
    fn from_row(row: PgRow) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            owner_hash: row.get("owner_hash"),
            created_at: row.get("created_at"),
            epoch: row.get("epoch"),
            max_members: row.get("max_members"),
            is_public: row.get("is_public"),
        }
    }
}

/// Group member model
#[derive(Debug, Clone)]
pub(crate) struct GroupMember {
    pub member_hash: String,
    pub is_admin: bool,
    pub joined_at: DateTime<Utc>,
}

impl GroupMember {
    fn from_row(row: &PgRow) -> Self {
        Self {
            member_hash: row.get("member_hash"),
            is_admin: row.get("is_admin"),
            joined_at: row.get("joined_at"),
        }
    }
}

// ============================================================================
// CALL OPERATIONS
// ============================================================================

/// Call repository
pub(crate) struct CallRepository<'a> {
    db: &'a Database,
}

impl<'a> CallRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new call session
    pub(crate) async fn create(
        &self,
        initiator_hash: &str,
        recipient_hash: &str,
        call_type: &str,
    ) -> Result<uuid::Uuid, SqlxError> {
        let id: uuid::Uuid = sqlx::query_scalar(r#"
            INSERT INTO call_sessions (initiator_hash, recipient_hash, call_type)
            VALUES ($1, $2, $3)
            RETURNING id
        "#)
        .bind(initiator_hash)
        .bind(recipient_hash)
        .bind(call_type)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(id)
    }

    /// Update call status
    pub(crate) async fn update_status(
        &self,
        call_id: uuid::Uuid,
        status: &str,
    ) -> Result<(), SqlxError> {
        match status {
            "active" => {
                sqlx::query(r#"
                    UPDATE call_sessions 
                    SET status = $2, answered_at = NOW()
                    WHERE id = $1
                "#)
                .bind(call_id)
                .bind(status)
                .execute(&self.db.pool)
                .await?;
            }
            "ended" | "missed" => {
                sqlx::query(r#"
                    UPDATE call_sessions 
                    SET status = $2, ended_at = NOW()
                    WHERE id = $1
                "#)
                .bind(call_id)
                .bind(status)
                .execute(&self.db.pool)
                .await?;
            }
            _ => {
                sqlx::query(r#"
                    UPDATE call_sessions SET status = $2 WHERE id = $1
                "#)
                .bind(call_id)
                .bind(status)
                .execute(&self.db.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Get call by ID
    pub(crate) async fn get(&self, call_id: uuid::Uuid) -> Result<Option<CallSession>, SqlxError> {
        let row = sqlx::query(r#"
            SELECT id, initiator_hash, recipient_hash, call_type, status,
                   started_at, answered_at, ended_at
            FROM call_sessions
            WHERE id = $1
        "#)
        .bind(call_id)
        .fetch_optional(&self.db.pool)
        .await?;

        Ok(row.map(|r| CallSession::from_row(&r)))
    }

    /// Get active calls for user
    pub(crate) async fn active_calls(&self, user_hash: &str) -> Result<Vec<CallSession>, SqlxError> {
        let rows = sqlx::query(r#"
            SELECT id, initiator_hash, recipient_hash, call_type, status,
                   started_at, answered_at, ended_at
            FROM call_sessions
            WHERE (initiator_hash = $1 OR recipient_hash = $1)
              AND status IN ('ringing', 'active')
        "#)
        .bind(user_hash)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(rows.iter().map(CallSession::from_row).collect())
    }
}

/// Call session model
#[derive(Debug, Clone)]
pub(crate) struct CallSession {
    pub id: uuid::Uuid,
    pub initiator_hash: String,
    pub recipient_hash: String,
    pub call_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub answered_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl CallSession {
    fn from_row(row: &PgRow) -> Self {
        Self {
            id: row.get("id"),
            initiator_hash: row.get("initiator_hash"),
            recipient_hash: row.get("recipient_hash"),
            call_type: row.get("call_type"),
            status: row.get("status"),
            started_at: row.get("started_at"),
            answered_at: row.get("answered_at"),
            ended_at: row.get("ended_at"),
        }
    }
}

// ============================================================================
// AUDIT OPERATIONS
// ============================================================================

/// Audit repository
pub(crate) struct AuditRepository<'a> {
    db: &'a Database,
}

impl<'a> AuditRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Log an audit event
    pub(crate) async fn log(
        &self,
        event_type: &str,
        user_hash: Option<&str>,
        ip_address: Option<&str>,
        details: Option<serde_json::Value>,
    ) -> Result<i64, SqlxError> {
        let id: i64 = sqlx::query_scalar(r#"
            INSERT INTO audit_log (event_type, user_hash, ip_address, details)
            VALUES ($1, $2, $3::INET, $4)
            RETURNING id
        "#)
        .bind(event_type)
        .bind(user_hash)
        .bind(ip_address)
        .bind(details)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(id)
    }

    /// Get audit logs for user
    pub(crate) async fn user_logs(
        &self,
        user_hash: &str,
        limit: i64,
    ) -> Result<Vec<AuditEntry>, SqlxError> {
        let rows = sqlx::query(r#"
            SELECT id, event_type, user_hash, ip_address, details, created_at
            FROM audit_log
            WHERE user_hash = $1
            ORDER BY created_at DESC
            LIMIT $2
        "#)
        .bind(user_hash)
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(rows.iter().map(AuditEntry::from_row).collect())
    }

    /// Cleanup old logs
    pub(crate) async fn cleanup(&self, days: i32) -> Result<u64, SqlxError> {
        let result = sqlx::query(r#"
            DELETE FROM audit_log 
            WHERE created_at < NOW() - ($1 || ' days')::INTERVAL
        "#)
        .bind(days.to_string())
        .execute(&self.db.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Audit entry model
#[derive(Debug, Clone)]
pub(crate) struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub user_hash: Option<String>,
    pub ip_address: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl AuditEntry {
    fn from_row(row: &PgRow) -> Self {
        Self {
            id: row.get("id"),
            event_type: row.get("event_type"),
            user_hash: row.get("user_hash"),
            ip_address: row.get("ip_address"),
            details: row.get("details"),
            created_at: row.get("created_at"),
        }
    }
}

// ============================================================================
// SECURITY ALERT OPERATIONS
// ============================================================================

/// Security alert repository
pub(crate) struct AlertRepository<'a> {
    db: &'a Database,
}

impl<'a> AlertRepository<'a> {
    pub(crate) fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a security alert
    pub(crate) async fn create(
        &self,
        severity: &str,
        alert_type: &str,
        description: &str,
        user_hash: Option<&str>,
        ip_address: Option<&str>,
        anomaly_score: Option<f64>,
        features: Option<serde_json::Value>,
    ) -> Result<uuid::Uuid, SqlxError> {
        let id: uuid::Uuid = sqlx::query_scalar(r#"
            INSERT INTO security_alerts (severity, alert_type, description, user_hash, ip_address, anomaly_score, features)
            VALUES ($1, $2, $3, $4, $5::INET, $6, $7)
            RETURNING id
        "#)
        .bind(severity)
        .bind(alert_type)
        .bind(description)
        .bind(user_hash)
        .bind(ip_address)
        .bind(anomaly_score)
        .bind(features)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(id)
    }

    /// Get recent alerts
    pub(crate) async fn recent(&self, limit: i64) -> Result<Vec<SecurityAlert>, SqlxError> {
        let rows = sqlx::query(r#"
            SELECT id, severity, alert_type, description, user_hash, ip_address,
                   anomaly_score, features, created_at, resolved_at
            FROM security_alerts
            ORDER BY created_at DESC
            LIMIT $1
        "#)
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await?;

        Ok(rows.iter().map(SecurityAlert::from_row).collect())
    }

    /// Resolve an alert
    pub(crate) async fn resolve(&self, alert_id: uuid::Uuid) -> Result<(), SqlxError> {
        sqlx::query(r#"
            UPDATE security_alerts SET resolved_at = NOW() WHERE id = $1
        "#)
        .bind(alert_id)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Get unresolved alerts count
    pub(crate) async fn unresolved_count(&self) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(r#"
            SELECT COUNT(*) FROM security_alerts WHERE resolved_at IS NULL
        "#)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(count)
    }
}

/// Security alert model
#[derive(Debug, Clone)]
pub(crate) struct SecurityAlert {
    pub id: uuid::Uuid,
    pub severity: String,
    pub alert_type: String,
    pub description: String,
    pub user_hash: Option<String>,
    pub ip_address: Option<String>,
    pub anomaly_score: Option<f64>,
    pub features: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl SecurityAlert {
    fn from_row(row: &PgRow) -> Self {
        Self {
            id: row.get("id"),
            severity: row.get("severity"),
            alert_type: row.get("alert_type"),
            description: row.get("description"),
            user_hash: row.get("user_hash"),
            ip_address: row.get("ip_address"),
            anomaly_score: row.get("anomaly_score"),
            features: row.get("features"),
            created_at: row.get("created_at"),
            resolved_at: row.get("resolved_at"),
        }
    }
}

// ============================================================================
// DATABASE STATISTICS
// ============================================================================

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct DatabaseStats {
    pub total_users: i64,
    pub online_users: i64,
    pub total_messages: i64,
    pub queued_messages: i64,
    pub total_groups: i64,
    pub active_calls: i64,
    pub unresolved_alerts: i64,
}

/// Get database statistics
pub(crate) async fn get_stats(db: &Database) -> Result<DatabaseStats, SqlxError> {
    let (total_users, online_users, queued_messages, total_groups, active_calls, unresolved_alerts) = tokio::join!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users").fetch_one(db.pool()),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE status = 'online' AND last_seen > NOW() - INTERVAL '5 minutes'").fetch_one(db.pool()),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM offline_messages WHERE is_delivered = FALSE AND expires_at > NOW()").fetch_one(db.pool()),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM groups").fetch_one(db.pool()),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM call_sessions WHERE status IN ('ringing', 'active')").fetch_one(db.pool()),
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM security_alerts WHERE resolved_at IS NULL").fetch_one(db.pool()),
    );

    Ok(DatabaseStats {
        total_users: total_users?,
        online_users: online_users?,
        total_messages: 0, // Would need a messages table
        queued_messages: queued_messages?,
        total_groups: total_groups?,
        active_calls: active_calls?,
        unresolved_alerts: unresolved_alerts?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_database_operations() {
        let config = DatabaseConfig::default();
        let db = Database::new(config).await.unwrap();
        
        // Test migration
        db.migrate().await.unwrap();
        
        // Test health check
        assert!(db.health_check().await.unwrap());
    }
}
