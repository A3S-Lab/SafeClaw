//! Key management for secure communication
//!
//! **Threat model**: Defends against A3 (network attacker) and A5 (insider) at AS-5.
//! See `docs/threat-model.md` §4 AS-5, §5.

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::{Zeroize, ZeroizeOnDrop};

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

/// Ephemeral X25519 key pair for one-time key exchange (forward secrecy).
///
/// Each session generates a fresh ephemeral key pair. The secret is consumed
/// during Diffie-Hellman and cannot be reused, ensuring forward secrecy:
/// compromising a long-term key does not reveal past session keys.
pub struct EphemeralKeyPair {
    secret: Option<EphemeralSecret>,
    public: PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new ephemeral key pair
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey(X25519PublicKey::from(&secret));
        Self {
            secret: Some(secret),
            public,
        }
    }

    /// Get the public key (safe to share)
    pub fn public_key(&self) -> PublicKey {
        self.public.clone()
    }

    /// Perform Diffie-Hellman and consume the ephemeral secret.
    ///
    /// Returns the shared secret. The ephemeral private key is destroyed
    /// after this call (forward secrecy).
    pub fn diffie_hellman(mut self, their_public: &PublicKey) -> Result<SharedSecret, &'static str> {
        let secret = self.secret.take().ok_or("Ephemeral secret already consumed")?;
        let shared = secret.diffie_hellman(&their_public.0);
        Ok(SharedSecret(shared.to_bytes()))
    }
}

/// Shared secret from Diffie-Hellman key exchange.
///
/// Zeroized on drop to prevent secret material from lingering in memory.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SharedSecret(pub(crate) [u8; 32]);

impl SharedSecret {
    /// Access the raw bytes (for key derivation only)
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Ed25519 key pair for signing (long-term identity key).
///
/// Used only for identity verification and message signing,
/// NOT for key exchange. Key exchange uses ephemeral X25519 keys.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ephemeral_key_exchange() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();

        let alice_pub = alice.public_key();
        let bob_pub = bob.public_key();

        let alice_shared = alice.diffie_hellman(&bob_pub).unwrap();
        let bob_shared = bob.diffie_hellman(&alice_pub).unwrap();

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_ephemeral_secret_consumed() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let bob_pub = bob.public_key();

        // First DH succeeds
        let _shared = alice.diffie_hellman(&bob_pub).unwrap();

        // alice is moved — cannot be used again (compile-time guarantee)
    }

    #[test]
    fn test_signing() {
        let keypair = KeyPair::generate();
        let message = b"Test message";

        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_public_key_roundtrip() {
        let kp = EphemeralKeyPair::generate();
        let pk = kp.public_key();
        let bytes = *pk.as_bytes();
        let pk2 = PublicKey::from_bytes(&bytes);
        assert_eq!(pk.as_bytes(), pk2.as_bytes());
    }

    #[test]
    fn test_shared_secret_zeroize() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let bob_pub = bob.public_key();

        let shared = alice.diffie_hellman(&bob_pub).unwrap();
        // Verify it has content
        assert_ne!(shared.as_bytes(), &[0u8; 32]);
        // Drop will zeroize — can't test post-drop, but Zeroize derive ensures it
    }
}
