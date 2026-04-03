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

// Fine-grained permissions & access control management
// nexus-relay/src/access_control.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub(crate) enum Permission {
    // Message permissions
    SendMessage,
    ReadMessage,
    DeleteMessage,
    ForwardMessage,
    ReactToMessage,
    EditMessage,

    // Group permissions
    CreateGroup,
    EditGroup,
    DeleteGroup,
    ManageMembers,
    ManageRoles,
    ViewMembers,

    // Call permissions
    InitiateCall,
    ReceiveCall,
    RecordCall,
    ScreenShare,

    // File permissions
    UploadFile,
    DownloadFile,
    DeleteFile,
    ShareFile,

    // User permissions
    ViewProfile,
    EditProfile,
    ManageSettings,
    ExportData,
    DeleteAccount,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub(crate) enum Role {
    Owner,
    Admin,
    Moderator,
    Member,
    Guest,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AccessControlEntry {
    pub id: String,
    pub subject_id: String,
    pub subject_type: String, // "user" or "role"
    pub resource_id: String,
    pub resource_type: String,
    pub permissions: HashSet<Permission>,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conditions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RoleDefinition {
    pub id: String,
    pub name: String,
    pub permissions: HashSet<Permission>,
    pub created_at: DateTime<Utc>,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UserRole {
    pub user_id: String,
    pub role: Role,
    pub resource_id: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,
}

pub(crate) struct AccessControlService {
    access_entries: HashMap<String, AccessControlEntry>,
    role_definitions: HashMap<String, RoleDefinition>,
    user_roles: HashMap<String, UserRole>,
    default_roles: HashMap<Role, HashSet<Permission>>,
}

impl AccessControlService {
    pub(crate) fn new() -> Self {
        let mut default_roles = HashMap::new();

        // Owner has all permissions
        let mut owner_perms = HashSet::new();
        owner_perms.insert(Permission::SendMessage);
        owner_perms.insert(Permission::ReadMessage);
        owner_perms.insert(Permission::DeleteMessage);
        owner_perms.insert(Permission::CreateGroup);
        owner_perms.insert(Permission::EditGroup);
        owner_perms.insert(Permission::DeleteGroup);
        owner_perms.insert(Permission::ManageMembers);
        owner_perms.insert(Permission::ManageRoles);
        default_roles.insert(Role::Owner, owner_perms);

        // Admin has most permissions except deletion
        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::SendMessage);
        admin_perms.insert(Permission::ReadMessage);
        admin_perms.insert(Permission::EditGroup);
        admin_perms.insert(Permission::ManageMembers);
        admin_perms.insert(Permission::ManageRoles);
        admin_perms.insert(Permission::ViewMembers);
        default_roles.insert(Role::Admin, admin_perms);

        // Moderator has limited permissions
        let mut mod_perms = HashSet::new();
        mod_perms.insert(Permission::SendMessage);
        mod_perms.insert(Permission::ReadMessage);
        mod_perms.insert(Permission::ViewMembers);
        default_roles.insert(Role::Moderator, mod_perms);

        // Member has basic permissions
        let mut member_perms = HashSet::new();
        member_perms.insert(Permission::SendMessage);
        member_perms.insert(Permission::ReadMessage);
        member_perms.insert(Permission::ViewProfile);
        default_roles.insert(Role::Member, member_perms);

        // Guest has minimal permissions
        let mut guest_perms = HashSet::new();
        guest_perms.insert(Permission::ReadMessage);
        guest_perms.insert(Permission::ViewProfile);
        default_roles.insert(Role::Guest, guest_perms);

        AccessControlService {
            access_entries: HashMap::new(),
            role_definitions: HashMap::new(),
            user_roles: HashMap::new(),
            default_roles,
        }
    }

    pub(crate) fn grant_permission(
        &mut self,
        user_id: &str,
        resource_id: &str,
        permission: Permission,
        granted_by: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<AccessControlEntry, String> {
        let mut permissions = HashSet::new();
        permissions.insert(permission);

        let entry = AccessControlEntry {
            id: format!("ace_{}", uuid::Uuid::new_v4()),
            subject_id: user_id.to_string(),
            subject_type: "user".to_string(),
            resource_id: resource_id.to_string(),
            resource_type: "resource".to_string(),
            permissions,
            granted_by: granted_by.to_string(),
            granted_at: Utc::now(),
            expires_at,
            conditions: vec![],
        };

        self.access_entries
            .insert(entry.id.clone(), entry.clone());
        Ok(entry)
    }

    pub(crate) fn revoke_permission(
        &mut self,
        user_id: &str,
        resource_id: &str,
        permission: Permission,
    ) -> Result<(), String> {
        // Find and update entry
        let mut found = false;
        for entry in self.access_entries.values_mut() {
            if entry.subject_id == user_id && entry.resource_id == resource_id {
                entry.permissions.remove(&permission);
                found = true;
            }
        }

        if found {
            Ok(())
        } else {
            Err("Access entry not found".to_string())
        }
    }

    pub(crate) fn assign_role(
        &mut self,
        user_id: &str,
        role: Role,
        resource_id: &str,
        assigned_by: &str,
    ) -> UserRole {
        let user_role = UserRole {
            user_id: user_id.to_string(),
            role: role.clone(),
            resource_id: resource_id.to_string(),
            assigned_at: Utc::now(),
            assigned_by: assigned_by.to_string(),
        };

        let key = format!("{}:{}", user_id, resource_id);
        self.user_roles.insert(key, user_role.clone());

        // Grant permissions based on role
        let permissions_to_grant: Vec<Permission> = if let Some(perms) = self.default_roles.get(&role) {
            perms.iter().cloned().collect()
        } else {
            Vec::new()
        };

        for permission in permissions_to_grant {
            let _ = self.grant_permission(user_id, resource_id, permission, assigned_by, None);
        }

        user_role
    }

    pub(crate) fn check_permission(
        &self,
        user_id: &str,
        resource_id: &str,
        permission: &Permission,
    ) -> bool {
        // Check direct access entries
        for entry in self.access_entries.values() {
            if entry.subject_id == user_id && entry.resource_id == resource_id
                && entry.permissions.contains(permission) {
                // Check if entry is expired
                if let Some(expires_at) = entry.expires_at {
                    if expires_at > Utc::now() {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }

        // Check role-based permissions
        let key = format!("{}:{}", user_id, resource_id);
        if let Some(user_role) = self.user_roles.get(&key) {
            if let Some(perms) = self.default_roles.get(&user_role.role) {
                return perms.contains(permission);
            }
        }

        false
    }

    pub(crate) fn get_user_permissions(
        &self,
        user_id: &str,
        resource_id: &str,
    ) -> HashSet<Permission> {
        let mut permissions = HashSet::new();

        // Collect from direct entries
        for entry in self.access_entries.values() {
            if entry.subject_id == user_id && entry.resource_id == resource_id {
                if let Some(expires_at) = entry.expires_at {
                    if expires_at > Utc::now() {
                        permissions.extend(entry.permissions.iter().cloned());
                    }
                } else {
                    permissions.extend(entry.permissions.iter().cloned());
                }
            }
        }

        // Collect from role
        let key = format!("{}:{}", user_id, resource_id);
        if let Some(user_role) = self.user_roles.get(&key) {
            if let Some(perms) = self.default_roles.get(&user_role.role) {
                permissions.extend(perms.iter().cloned());
            }
        }

        permissions
    }

    pub(crate) fn create_custom_permission_set(
        &mut self,
        name: &str,
        permissions: HashSet<Permission>,
        description: &str,
    ) -> RoleDefinition {
        let role_def = RoleDefinition {
            id: format!("role_{}", uuid::Uuid::new_v4()),
            name: name.to_string(),
            permissions,
            created_at: Utc::now(),
            description: description.to_string(),
        };

        self.role_definitions
            .insert(role_def.id.clone(), role_def.clone());
        role_def
    }

    pub(crate) fn audit_access(&self, user_id: &str) -> Vec<AccessControlEntry> {
        self.access_entries
            .values()
            .filter(|e| e.subject_id == user_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_permission() {
        let mut service = AccessControlService::new();
        let result = service.grant_permission(
            "user_1",
            "resource_1",
            Permission::SendMessage,
            "admin",
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_check_permission() {
        let mut service = AccessControlService::new();
        service.grant_permission(
            "user_1",
            "resource_1",
            Permission::SendMessage,
            "admin",
            None,
        ).ok();

        assert!(service.check_permission("user_1", "resource_1", &Permission::SendMessage));
        assert!(!service.check_permission("user_1", "resource_1", &Permission::DeleteMessage));
    }

    #[test]
    fn test_assign_role() {
        let mut service = AccessControlService::new();
        service.assign_role("user_1", Role::Admin, "resource_1", "admin");

        assert!(service.check_permission("user_1", "resource_1", &Permission::SendMessage));
        assert!(service.check_permission("user_1", "resource_1", &Permission::ManageMembers));
    }

    #[test]
    fn test_revoke_permission() {
        let mut service = AccessControlService::new();
        service
            .grant_permission("user_1", "resource_1", Permission::SendMessage, "admin", None)
            .ok();

        let result = service.revoke_permission("user_1", "resource_1", Permission::SendMessage);
        assert!(result.is_ok());
        assert!(!service.check_permission("user_1", "resource_1", &Permission::SendMessage));
    }

    #[test]
    fn test_get_user_permissions() {
        let mut service = AccessControlService::new();
        service.grant_permission(
            "user_1",
            "resource_1",
            Permission::SendMessage,
            "admin",
            None,
        ).ok();
        service.grant_permission(
            "user_1",
            "resource_1",
            Permission::ReadMessage,
            "admin",
            None,
        ).ok();

        let perms = service.get_user_permissions("user_1", "resource_1");
        assert_eq!(perms.len(), 2);
    }

    #[test]
    fn test_permission_expiry() {
        let mut service = AccessControlService::new();
        service.grant_permission(
            "user_1",
            "resource_1",
            Permission::SendMessage,
            "admin",
            Some(Utc::now() - chrono::Duration::hours(1)),
        ).ok();

        assert!(!service.check_permission("user_1", "resource_1", &Permission::SendMessage));
    }

    #[test]
    fn test_audit_access() {
        let mut service = AccessControlService::new();
        service.grant_permission(
            "user_1",
            "resource_1",
            Permission::SendMessage,
            "admin",
            None,
        ).ok();

        let audit = service.audit_access("user_1");
        assert_eq!(audit.len(), 1);
    }
}
