// SPDX-License-Identifier: AGPL-3.0-only
//! Content hashes. Only SHA-256 is defined so far; legacy hashes (SHA-1, zsync/MD4) arrive with the compatibility
//! layer and are subject to the rights conditions for Jazsync/MD4 (RQ-02).

use std::fmt;

/// Supported hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HashAlgorithm {
    /// SHA-256 (64 hex digits).
    Sha256,
}

impl HashAlgorithm {
    /// Number of hex digits of a digest.
    #[must_use]
    pub const fn hex_len(self) -> usize {
        match self {
            Self::Sha256 => 64,
        }
    }

    /// Stable lower-case algorithm name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }
}

/// A malformed digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashError {
    /// The digest has the wrong number of hex digits.
    WrongLength,
    /// The digest contains a non-hex character.
    NotHex,
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WrongLength => "digest has the wrong length",
            Self::NotHex => "digest contains non-hex characters",
        })
    }
}

impl std::error::Error for HashError {}

/// An algorithm plus a lower-case hex digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash {
    algorithm: HashAlgorithm,
    hex: String,
}

impl ContentHash {
    /// Parses a hex digest (case-insensitive input, stored lower-case).
    ///
    /// # Errors
    /// [`HashError`] if the length or the characters are wrong.
    pub fn new(algorithm: HashAlgorithm, hex: &str) -> Result<Self, HashError> {
        if hex.len() != algorithm.hex_len() {
            return Err(HashError::WrongLength);
        }
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HashError::NotHex);
        }
        Ok(Self {
            algorithm,
            hex: hex.to_ascii_lowercase(),
        })
    }

    /// Builds a digest from raw bytes.
    ///
    /// # Errors
    /// [`HashError::WrongLength`] if `bytes` does not match the algorithm's digest size.
    pub fn from_bytes(algorithm: HashAlgorithm, bytes: &[u8]) -> Result<Self, HashError> {
        use fmt::Write as _;
        let mut hex = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
        }
        Self::new(algorithm, &hex)
    }

    /// Builds a SHA-256 hash from a raw 32-byte digest (infallible).
    #[must_use]
    pub fn from_sha256_digest(digest: [u8; 32]) -> Self {
        use fmt::Write as _;
        let mut hex = String::with_capacity(64);
        for byte in digest {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
        }
        Self {
            algorithm: HashAlgorithm::Sha256,
            hex,
        }
    }

    /// The algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// The lower-case hex digest.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.hex
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm.name(), self.hex)
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentHash, HashAlgorithm, HashError};

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn parses_and_normalises_case() {
        let hash = ContentHash::new(HashAlgorithm::Sha256, &EMPTY.to_uppercase()).unwrap();
        assert_eq!(hash.hex(), EMPTY);
        assert_eq!(hash.to_string(), format!("sha256:{EMPTY}"));
    }

    #[test]
    fn rejects_malformed_digests() {
        assert_eq!(
            ContentHash::new(HashAlgorithm::Sha256, "abc").unwrap_err(),
            HashError::WrongLength
        );
        let bad = format!("{}g", &EMPTY[..63]);
        assert_eq!(
            ContentHash::new(HashAlgorithm::Sha256, &bad).unwrap_err(),
            HashError::NotHex
        );
    }

    #[test]
    fn from_bytes_round_trips() {
        let bytes: Vec<u8> = (0..32).collect();
        let hash = ContentHash::from_bytes(HashAlgorithm::Sha256, &bytes).unwrap();
        assert!(hash.hex().starts_with("000102"));
        assert_eq!(
            ContentHash::from_bytes(HashAlgorithm::Sha256, &bytes[..5]).unwrap_err(),
            HashError::WrongLength
        );
    }
}
