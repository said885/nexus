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

pub mod error;
pub mod handler;
pub mod state;
pub mod tls;
pub mod api;
pub mod metrics;
pub mod federation;
pub mod plugins;
pub mod groups;
pub mod call_encryption;
pub mod sync;
pub mod accounts;
pub mod push_notifications;
pub mod message_search;
mod media_storage;
pub mod audit;
pub mod reactions;
pub mod voice_messages;
pub mod presence;
mod drafts;
mod rate_limiting;
pub mod notifications;
pub mod backup;
mod scheduling;
pub mod encryption_manager;
mod secure_deletion;
mod metadata_privacy;
mod threat_detection;
mod access_control;
