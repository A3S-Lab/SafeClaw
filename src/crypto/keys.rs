//! Key management for secure communication

use crate::error::{Error, Result};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, ReusableSecret};

/// X25519 public key for key exchange
#[derive(Clone)]
pub struct PublicKey(pub(crate) X25519PublicKey);

impl PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(X25519PublicKey::from(*bytes))
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

/// X25519 secret key for key exchange
pub struct SecretKey(pub(crate) ReusableSecret);

impl SecretKey {
    /// Generate a new random secret key
    pub fn generate() -> Self {
        Self(ReusableSecret::random_from_rng(OsRng))
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &PublicKey) -> [u8; 32] {
        self.0.diffie_hellman(&their_public.0).to_bytes()
    }

    /// Get the corresponding public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(X25519PublicKey::from(&self.0))
    }
}

/// Ed25519 key pair for signing
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Get the verifying (public) key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the verifying key as bytes
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }
}

/// Ephemeral key pair for one-time key exchange
pub struct EphemeralKeyPair {
    secret: EphemeralSecret,
    public: X25519PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new ephemeral key pair
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.public)
    }

    /// Perform Diffie-Hellman and consume the ephemeral secret
    pub fn diffie_hellman(self, their_public: &PublicKey) -> [u8; 32] {
        self.secret.diffie_hellman(&their_public.0).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange() {
        let alice_secret = SecretKey::generate();
        let alice_public = alice_secret.public_key();

        let bob_secret = SecretKey::generate();
        let bob_public = bob_secret.public_key();

        let alice_shared = alice_secret.diffie_hellman(&bob_public);
        let bob_shared = bob_secret.diffie_hellman(&alice_public);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_ephemeral_key_exchange() {
        let alice = EphemeralKeyPair::generate();
        let alice_public = alice.public_key();

        let bob_secret = SecretKey::generate();
        let bob_public = bob_secret.public_key();

        let alice_shared = alice.diffie_hellman(&bob_public);
        let bob_shared = bob_secret.diffie_hellman(&alice_public);

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_signing() {
        let keypair = KeyPair::generate();
        let message = b"Test message";

        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64);
    }
}
