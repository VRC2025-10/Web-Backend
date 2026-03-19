// Session token hashing utilities.
//
// Centralises the SHA-256 hashing of raw session tokens so that
// (a) the algorithm is defined in exactly one place, and
// (b) Kani can formally verify properties of the hash function.

use sha2::{Digest, Sha256};

/// Compute the SHA-256 hash of a raw session token.
///
/// The returned `Vec<u8>` is the 32-byte digest stored in the database.
/// Raw tokens are **never** persisted — only this hash is stored.
pub fn sha256_hash(token: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(token);
    hasher.finalize().to_vec()
}

// ---------------------------------------------------------------------------
// Kani formal verification harnesses for session token hashing (P4).
// Run with: cargo kani --harness proof_different_tokens_different_hashes
// ---------------------------------------------------------------------------
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// P4a: Within the bounded model-checking domain, different tokens
    /// always produce different SHA-256 hashes (collision freedom).
    #[kani::proof]
    fn proof_different_tokens_different_hashes() {
        let token1: [u8; 32] = kani::any();
        let token2: [u8; 32] = kani::any();
        kani::assume(token1 != token2);

        let hash1 = sha256_hash(&token1);
        let hash2 = sha256_hash(&token2);

        assert!(hash1 != hash2, "SHA-256 must not collide for distinct 32-byte tokens");
    }

    /// P4b: SHA-256 is deterministic — the same token always yields the
    /// same hash.
    #[kani::proof]
    fn proof_hash_is_deterministic() {
        let token: [u8; 32] = kani::any();
        let hash1 = sha256_hash(&token);
        let hash2 = sha256_hash(&token);
        assert_eq!(hash1, hash2, "SHA-256 must be deterministic");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash_deterministic() {
        let token = [0xABu8; 32];
        assert_eq!(sha256_hash(&token), sha256_hash(&token));
    }

    #[test]
    fn test_sha256_hash_produces_32_bytes() {
        let token = [0u8; 32];
        assert_eq!(sha256_hash(&token).len(), 32);
    }

    #[test]
    fn test_sha256_hash_different_inputs() {
        let a = [0u8; 32];
        let b = [1u8; 32];
        assert_ne!(sha256_hash(&a), sha256_hash(&b));
    }
}
