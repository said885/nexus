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
