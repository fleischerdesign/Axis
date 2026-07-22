use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Decryption failed (authentication tag mismatch or invalid payload)")]
    DecryptionFailed,
    #[error("Ciphertext payload too short")]
    PayloadTooShort,
}

/// Derives a deterministic 256-bit symmetric session key from a 6-digit PIN and device IDs.
/// Device IDs are lexicographically sorted so both Sharer and Receiver compute the identical key.
pub fn derive_session_key(pin: &str, device_id_a: &str, device_id_b: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let (first, second) = if device_id_a <= device_id_b {
        (device_id_a, device_id_b)
    } else {
        (device_id_b, device_id_a)
    };

    let mut hasher = DefaultHasher::new();
    "axis-continuity-v1".hash(&mut hasher);
    pin.hash(&mut hasher);
    first.hash(&mut hasher);
    second.hash(&mut hasher);

    let h1 = hasher.finish();

    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        let mut h = DefaultHasher::new();
        h1.hash(&mut h);
        i.hash(&mut h);
        let val = h.finish();
        *byte = (val & 0xFF) as u8;
    }
    key
}

/// Authenticated symmetric cipher using ChaCha20-Poly1305.
pub struct ContinuityCipher {
    cipher: ChaCha20Poly1305,
    send_nonce_counter: u64,
}

impl ContinuityCipher {
    pub fn new(key_bytes: &[u8; 32]) -> Self {
        let key = Key::from_slice(key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        Self {
            cipher,
            send_nonce_counter: 1,
        }
    }

    /// Encrypts plaintext bytes and prepends a 12-byte nonce.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let mut nonce_bytes = [0u8; 12];
        let counter_bytes = self.send_nonce_counter.to_le_bytes();
        nonce_bytes[0..8].copy_from_slice(&counter_bytes);
        self.send_nonce_counter += 1;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .expect("ChaCha20Poly1305 encryption failure");

        let mut packet = Vec::with_capacity(12 + ciphertext.len());
        packet.extend_from_slice(&nonce_bytes);
        packet.extend_from_slice(&ciphertext);
        packet
    }

    /// Decrypts a packet (12-byte nonce + ciphertext).
    pub fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if packet.len() < 12 + 16 {
            return Err(CryptoError::PayloadTooShort);
        }
        let (nonce_bytes, ciphertext) = packet.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_derivation_is_symmetric() {
        let key1 = derive_session_key("123456", "device_alpha", "device_beta");
        let key2 = derive_session_key("123456", "device_beta", "device_alpha");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let key = derive_session_key("654321", "hostA", "hostB");
        let mut alice = ContinuityCipher::new(&key);
        let bob = ContinuityCipher::new(&key);

        let secret_msg = b"Hello Axis Continuity Encrypted Stream!";
        let encrypted = alice.encrypt(secret_msg);

        assert_ne!(&encrypted[12..], secret_msg);
        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, secret_msg);
    }

    #[test]
    fn test_tampered_ciphertext_fails_decryption() {
        let key = derive_session_key("111111", "hostA", "hostB");
        let mut alice = ContinuityCipher::new(&key);
        let bob = ContinuityCipher::new(&key);

        let mut encrypted = alice.encrypt(b"Sensitive Mouse Event");
        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0xFF; // Tamper tag

        assert!(bob.decrypt(&encrypted).is_err());
    }
}
