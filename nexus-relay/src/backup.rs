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

// Backup and recovery module
// nexus-relay/src/backup.rs

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct BackupConfiguration {
    pub user_id: String,
    pub backup_enabled: bool,
    pub auto_backup: bool,
    pub backup_frequency: BackupFrequency,
    pub last_backup: Option<DateTime<Utc>>,
    pub next_scheduled_backup: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum BackupFrequency {
    Daily,
    Weekly,
    Monthly,
    Manual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Backup {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub backup_type: BackupType,
    pub status: BackupStatus,
    pub encrypted: bool,
    pub location: String, // S3 bucket, etc.
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum BackupType {
    Full,
    Incremental,
    Differential,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum BackupStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecoveryKey {
    pub id: String,
    pub user_id: String,
    pub key: String, // Encrypted recovery key
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

pub(crate) struct BackupManager {
    configurations: HashMap<String, BackupConfiguration>,
    backups: HashMap<String, Backup>,
    recovery_keys: HashMap<String, RecoveryKey>,
}

impl BackupManager {
    pub(crate) fn new() -> Self {
        BackupManager {
            configurations: HashMap::new(),
            backups: HashMap::new(),
            recovery_keys: HashMap::new(),
        }
    }

    pub(crate) fn create_configuration(
        &mut self,
        user_id: &str,
        frequency: BackupFrequency,
    ) -> Result<BackupConfiguration, String> {
        let config = BackupConfiguration {
            user_id: user_id.to_string(),
            backup_enabled: true,
            auto_backup: true,
            backup_frequency: frequency,
            last_backup: None,
            next_scheduled_backup: Some(Utc::now() + Duration::days(1)),
        };

        self.configurations
            .insert(user_id.to_string(), config.clone());
        Ok(config)
    }

    pub(crate) fn create_backup(
        &mut self,
        user_id: &str,
        size_bytes: u64,
        backup_type: BackupType,
        encrypted: bool,
    ) -> Result<Backup, String> {
        let backup = Backup {
            id: format!("backup_{}", uuid::Uuid::new_v4()),
            user_id: user_id.to_string(),
            created_at: Utc::now(),
            size_bytes,
            backup_type,
            status: BackupStatus::InProgress,
            encrypted,
            location: format!("s3://nexus-backups/{}/{}", user_id, uuid::Uuid::new_v4()),
        };

        self.backups.insert(backup.id.clone(), backup.clone());

        // Update configuration
        if let Some(config) = self.configurations.get_mut(user_id) {
            config.last_backup = Some(Utc::now());
            config.next_scheduled_backup = Some(Utc::now() + Duration::days(1));
        }

        Ok(backup)
    }

    pub(crate) fn complete_backup(&mut self, backup_id: &str) -> Result<(), String> {
        if let Some(backup) = self.backups.get_mut(backup_id) {
            backup.status = BackupStatus::Complete;
            Ok(())
        } else {
            Err("Backup not found".to_string())
        }
    }

    pub(crate) fn fail_backup(&mut self, backup_id: &str, _error: &str) -> Result<(), String> {
        if let Some(backup) = self.backups.get_mut(backup_id) {
            backup.status = BackupStatus::Failed;
            Ok(())
        } else {
            Err("Backup not found".to_string())
        }
    }

    pub(crate) fn generate_recovery_key(
        &mut self,
        user_id: &str,
        key_material: &str,
    ) -> Result<RecoveryKey, String> {
        let recovery_key = RecoveryKey {
            id: format!("recovery_{}", uuid::Uuid::new_v4()),
            user_id: user_id.to_string(),
            key: key_material.to_string(),
            created_at: Utc::now(),
            used_at: None,
            expires_at: Utc::now() + Duration::days(90),
        };

        self.recovery_keys
            .insert(recovery_key.id.clone(), recovery_key.clone());
        Ok(recovery_key)
    }

    pub(crate) fn get_backups(&self, user_id: &str) -> Vec<&Backup> {
        self.backups
            .values()
            .filter(|b| b.user_id == user_id)
            .collect()
    }

    pub(crate) fn use_recovery_key(&mut self, recovery_key_id: &str) -> Result<(), String> {
        if let Some(key) = self.recovery_keys.get_mut(recovery_key_id) {
            if key.used_at.is_some() {
                return Err("Recovery key already used".to_string());
            }
            if Utc::now() > key.expires_at {
                return Err("Recovery key expired".to_string());
            }
            key.used_at = Some(Utc::now());
            Ok(())
        } else {
            Err("Recovery key not found".to_string())
        }
    }

    pub(crate) fn get_last_backup(&self, user_id: &str) -> Option<&Backup> {
        self.backups
            .values()
            .filter(|b| b.user_id == user_id && b.status == BackupStatus::Complete)
            .max_by_key(|b| b.created_at)
    }

    pub(crate) fn get_total_backup_size(&self, user_id: &str) -> u64 {
        self.backups
            .values()
            .filter(|b| b.user_id == user_id)
            .map(|b| b.size_bytes)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_configuration() {
        let mut manager = BackupManager::new();
        let result = manager.create_configuration("user_1", BackupFrequency::Weekly);
        assert!(result.is_ok());
        assert!(result.unwrap().backup_enabled);
    }

    #[test]
    fn test_create_backup() {
        let mut manager = BackupManager::new();
        let result = manager.create_backup("user_1", 1024000, BackupType::Full, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, BackupStatus::InProgress);
    }

    #[test]
    fn test_complete_backup() {
        let mut manager = BackupManager::new();
        let backup = manager
            .create_backup("user_1", 1024000, BackupType::Full, true)
            .unwrap();

        let result = manager.complete_backup(&backup.id);
        assert!(result.is_ok());
        assert_eq!(
            manager.backups.get(&backup.id).unwrap().status,
            BackupStatus::Complete
        );
    }

    #[test]
    fn test_generate_recovery_key() {
        let mut manager = BackupManager::new();
        let result = manager.generate_recovery_key("user_1", "key_material");
        assert!(result.is_ok());
        let key = result.unwrap();
        assert!(key.used_at.is_none());
    }

    #[test]
    fn test_use_recovery_key() {
        let mut manager = BackupManager::new();
        let key = manager
            .generate_recovery_key("user_1", "key_material")
            .unwrap();

        let result = manager.use_recovery_key(&key.id);
        assert!(result.is_ok());
        assert!(manager.recovery_keys.get(&key.id).unwrap().used_at.is_some());
    }

    #[test]
    fn test_get_last_backup() {
        let mut manager = BackupManager::new();
        let backup = manager
            .create_backup("user_1", 1024000, BackupType::Full, true)
            .unwrap();
        manager.complete_backup(&backup.id).unwrap();

        let last = manager.get_last_backup("user_1");
        assert!(last.is_some());
    }
}
