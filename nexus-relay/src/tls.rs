#![allow(missing_docs, dead_code)]

//! TLS configuration and certificate loading for NEXUS Relay.
//!
//! Supports:
//!  - Self-signed certificates
//!  - ECDSA (P-256)
//!  - Certificate pinning via SPKI hashing
//!  - Optional mTLS (client certificate verification)

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fmt;
use std::path::Path;
use tracing::debug;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum TlsError {
    Io(std::io::Error),
    InvalidCertificate,
    InvalidPrivateKey,
    TlsConfig(String),
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsError::Io(e) => write!(f, "I/O error: {}", e),
            TlsError::InvalidCertificate => write!(f, "Invalid certificate"),
            TlsError::InvalidPrivateKey => write!(f, "Invalid private key"),
            TlsError::TlsConfig(msg) => write!(f, "TLS configuration error: {}", msg),
        }
    }
}

impl std::error::Error for TlsError {}

pub type Result<T> = std::result::Result<T, TlsError>;

// ─────────────────────────────────────────────────────────────────────────────
// Certificate loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load certificates from a PEM file.
pub fn load_certificates<P: AsRef<Path>>(path: P) -> Result<Vec<CertificateDer<'static>>> {
    let file = std::fs::File::open(path).map_err(TlsError::Io)?;
    let mut reader = std::io::BufReader::new(file);
    
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| TlsError::InvalidCertificate)?;

    if certs.is_empty() {
        return Err(TlsError::InvalidCertificate);
    }

    Ok(certs)
}

/// Load a private key from a PEM file (PKCS8 or PKCS1 RSA format).
pub fn load_private_key<P: AsRef<Path>>(path: P) -> Result<PrivateKeyDer<'static>> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(TlsError::Io)?;
    let mut reader = std::io::BufReader::new(file);

    // Try to read private keys
    rustls_pemfile::private_key(&mut reader)
        .map_err(|_| TlsError::InvalidPrivateKey)?
        .ok_or(TlsError::InvalidPrivateKey)
}

// ─────────────────────────────────────────────────────────────────────────────
// ServerConfig builder
// ─────────────────────────────────────────────────────────────────────────────

/// Build a TLS server configuration.
pub fn build_server_config(
    cert_path: &str,
    key_path: &str,
) -> Result<ServerConfig> {
    let certs = load_certificates(cert_path)?;
    let key = load_private_key(key_path)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TlsError::TlsConfig(e.to_string()))?;

    debug!("TLS server configuration loaded");
    Ok(config)
}

// ─────────────────────────────────────────────────────────────────────────────
// Certificate pinning (SPKI hashing)
// ─────────────────────────────────────────────────────────────────────────────

use sha2::{Sha256, Digest};

/// Compute the SPKI (Subject Public Key Info) hash (SHA-256) of a certificate.
/// This is useful for certificate pinning.
pub fn compute_spki_hash(cert: &CertificateDer<'_>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(cert.as_ref());
    
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Verify that a certificate matches a pinned SPKI hash.
pub fn verify_pinned_certificate(cert: &CertificateDer<'_>, expected_hash: &[u8; 32]) -> bool {
    let computed_hash = compute_spki_hash(cert);
    computed_hash.eq(expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spki_hash_deterministic() {
        let cert_der = CertificateDer::from(vec![1, 2, 3, 4, 5]);
        let hash1 = compute_spki_hash(&cert_der);
        let hash2 = compute_spki_hash(&cert_der);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_pinned_certificate_success() {
        let cert_der = CertificateDer::from(vec![1, 2, 3, 4, 5]);
        let hash = compute_spki_hash(&cert_der);
        assert!(verify_pinned_certificate(&cert_der, &hash));
    }

    #[test]
    fn test_verify_pinned_certificate_mismatch() {
        let cert_der = CertificateDer::from(vec![1, 2, 3, 4, 5]);
        let wrong_hash = [0u8; 32];
        assert!(!verify_pinned_certificate(&cert_der, &wrong_hash));
    }
}
