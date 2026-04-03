// Copyright (c) 2026 said885 <frensh5@proton.me>
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NexusError {
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),
    #[error("Invalid key material")]
    InvalidKey,
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Session not initialized")]
    NoSession,
    #[error("Serialization error: {0}")]
    SerdeError(String),
    #[error("Message replay detected")]
    Replay,
    #[error("Too many skipped messages")]
    TooManySkipped,
    #[error("Integrity check failed")]
    IntegrityFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
}

pub type Result<T> = std::result::Result<T, NexusError>;
