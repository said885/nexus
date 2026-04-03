use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A fixed-size secret byte array that is zeroized on drop.
#[derive(Clone)]
pub struct SecretBytes<const N: usize>([u8; N]);

impl<const N: usize> SecretBytes<N> {
    /// Create a new SecretBytes from a fixed-size array.
    pub fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    /// Create a zeroed SecretBytes.
    pub fn zeroed() -> Self {
        Self([0u8; N])
    }

    /// Get a reference to the inner bytes.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }

    /// Get a mutable reference to the inner bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.0
    }

    /// Get a slice reference.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> Zeroize for SecretBytes<N> {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl<const N: usize> Drop for SecretBytes<N> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<const N: usize> ZeroizeOnDrop for SecretBytes<N> {}

impl<const N: usize> From<[u8; N]> for SecretBytes<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self(bytes)
    }
}

impl<const N: usize> AsRef<[u8]> for SecretBytes<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> std::fmt::Debug for SecretBytes<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretBytes<{}>([REDACTED])", N)
    }
}

/// A variable-length secret byte vector that is zeroized on drop.
#[derive(Clone)]
pub struct SecretVec(Vec<u8>);

impl SecretVec {
    /// Create a new `SecretVec` from a `Vec<u8>`.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Create an empty SecretVec.
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Create a zeroed SecretVec of the given length.
    pub fn zeroed(len: usize) -> Self {
        Self(vec![0u8; len])
    }

    /// Get a reference to the inner bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get a mutable reference to the inner bytes.
    pub fn as_bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    /// Get the length.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume and return the inner Vec (caller is responsible for zeroizing).
    pub fn into_vec(mut self) -> Vec<u8> {
        let mut v = Vec::new();
        std::mem::swap(&mut self.0, &mut v);
        // The zeroize of the now-empty self.0 is harmless; v is returned to caller.
        v
    }
}

impl Zeroize for SecretVec {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for SecretVec {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SecretVec {}

impl From<Vec<u8>> for SecretVec {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for SecretVec {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretVec([REDACTED, len={}])", self.0.len())
    }
}

/// Constant-time comparison of two byte slices.
/// Returns true if equal, false otherwise. Resistant to timing attacks.
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Length mismatch: still do a dummy comparison to avoid length-based timing.
        let dummy = vec![0u8; a.len()];
        let _ = a.ct_eq(&dummy);
        return false;
    }
    a.ct_eq(b).into()
}

/// Overwrite a byte slice with zeros.
pub fn secure_zero(buf: &mut [u8]) {
    buf.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_bytes_zeroize() {
        let mut sb = SecretBytes::<32>::new([0xAB; 32]);
        assert_eq!(sb.as_bytes(), &[0xAB; 32]);
        sb.zeroize();
        assert_eq!(sb.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_secret_vec_zeroize() {
        let mut sv = SecretVec::new(vec![0xFF; 16]);
        assert_eq!(sv.as_bytes(), &[0xFF; 16]);
        sv.zeroize();
        assert!(sv.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare(b"hello", b"hello"));
        assert!(!secure_compare(b"hello", b"world"));
        assert!(!secure_compare(b"hello", b"hell"));
        assert!(!secure_compare(b"", b"a"));
    }

    #[test]
    fn test_secure_zero() {
        let mut buf = [0xDE; 32];
        secure_zero(&mut buf);
        assert_eq!(buf, [0u8; 32]);
    }
}
