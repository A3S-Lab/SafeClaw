//! Secure channel for encrypted communication with TEE

use super::{decrypt, encrypt, generate_key, KEY_SIZE};
use crate::crypto::keys::{EphemeralKeyPair, PublicKey, SecretKey};
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Secure channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// Initial state, not yet established
    Initial,
    /// Handshake in progress
    Handshaking,
    /// Channel established and ready
    Established,
    /// Channel closed
    Closed,
}

/// Secure channel for encrypted communication
pub struct SecureChannel {
    state: Arc<RwLock<ChannelState>>,
    session_key: Arc<RwLock<Option<[u8; KEY_SIZE]>>>,
    local_secret: SecretKey,
    remote_public: Arc<RwLock<Option<PublicKey>>>,
    channel_id: String,
}

impl SecureChannel {
    /// Create a new secure channel
    pub fn new(channel_id: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(ChannelState::Initial)),
            session_key: Arc::new(RwLock::new(None)),
            local_secret: SecretKey::generate(),
            remote_public: Arc::new(RwLock::new(None)),
            channel_id,
        }
    }

    /// Get the channel ID
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// Get the local public key for handshake
    pub fn local_public_key(&self) -> PublicKey {
        self.local_secret.public_key()
    }

    /// Get current channel state
    pub async fn state(&self) -> ChannelState {
        *self.state.read().await
    }

    /// Start handshake by generating ephemeral keys
    pub async fn start_handshake(&self) -> Result<HandshakeInit> {
        let mut state = self.state.write().await;
        if *state != ChannelState::Initial {
            return Err(Error::Crypto("Channel not in initial state".to_string()));
        }

        *state = ChannelState::Handshaking;

        Ok(HandshakeInit {
            channel_id: self.channel_id.clone(),
            public_key: self.local_secret.public_key().as_bytes().to_vec(),
        })
    }

    /// Complete handshake with remote public key
    pub async fn complete_handshake(&self, remote_public_bytes: &[u8; 32]) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != ChannelState::Handshaking {
            return Err(Error::Crypto("Channel not in handshaking state".to_string()));
        }

        let remote_public = PublicKey::from_bytes(remote_public_bytes);

        // Perform key exchange
        let shared_secret = self.local_secret.diffie_hellman(&remote_public);

        // Derive session key using HKDF-like construction
        let session_key = derive_session_key(&shared_secret, &self.channel_id);

        // Store remote public key and session key
        *self.remote_public.write().await = Some(remote_public);
        *self.session_key.write().await = Some(session_key);
        *state = ChannelState::Established;

        Ok(())
    }

    /// Encrypt a message for sending
    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let state = self.state.read().await;
        if *state != ChannelState::Established {
            return Err(Error::Crypto("Channel not established".to_string()));
        }

        let session_key = self.session_key.read().await;
        let key = session_key
            .as_ref()
            .ok_or_else(|| Error::Crypto("No session key".to_string()))?;

        encrypt(key, plaintext)
    }

    /// Decrypt a received message
    pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let state = self.state.read().await;
        if *state != ChannelState::Established {
            return Err(Error::Crypto("Channel not established".to_string()));
        }

        let session_key = self.session_key.read().await;
        let key = session_key
            .as_ref()
            .ok_or_else(|| Error::Crypto("No session key".to_string()))?;

        decrypt(key, ciphertext)
    }

    /// Close the channel
    pub async fn close(&self) {
        let mut state = self.state.write().await;
        *state = ChannelState::Closed;

        // Clear session key
        *self.session_key.write().await = None;
    }
}

/// Handshake initialization message
#[derive(Debug, Clone)]
pub struct HandshakeInit {
    /// Channel identifier
    pub channel_id: String,
    /// Local public key
    pub public_key: Vec<u8>,
}

/// Builder for secure channels
pub struct SecureChannelBuilder {
    channel_id: Option<String>,
}

impl SecureChannelBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { channel_id: None }
    }

    /// Set the channel ID
    pub fn channel_id(mut self, id: impl Into<String>) -> Self {
        self.channel_id = Some(id.into());
        self
    }

    /// Build the secure channel
    pub fn build(self) -> Result<SecureChannel> {
        let channel_id = self
            .channel_id
            .ok_or_else(|| Error::Crypto("Channel ID required".to_string()))?;

        Ok(SecureChannel::new(channel_id))
    }
}

impl Default for SecureChannelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive session key from shared secret
fn derive_session_key(shared_secret: &[u8; 32], context: &str) -> [u8; KEY_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(context.as_bytes());
    hasher.update(b"safeclaw-session-key-v1");

    let result = hasher.finalize();
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secure_channel_handshake() {
        let channel1 = SecureChannel::new("test-channel".to_string());
        let channel2 = SecureChannel::new("test-channel".to_string());

        // Start handshake on both sides
        let init1 = channel1.start_handshake().await.unwrap();
        let init2 = channel2.start_handshake().await.unwrap();

        // Complete handshake
        let pk1: [u8; 32] = init1.public_key.try_into().unwrap();
        let pk2: [u8; 32] = init2.public_key.try_into().unwrap();

        channel1.complete_handshake(&pk2).await.unwrap();
        channel2.complete_handshake(&pk1).await.unwrap();

        // Test encryption/decryption
        let message = b"Hello, TEE!";
        let encrypted = channel1.encrypt(message).await.unwrap();
        let decrypted = channel2.decrypt(&encrypted).await.unwrap();

        assert_eq!(message.as_slice(), decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_channel_state_transitions() {
        let channel = SecureChannel::new("test".to_string());

        assert_eq!(channel.state().await, ChannelState::Initial);

        channel.start_handshake().await.unwrap();
        assert_eq!(channel.state().await, ChannelState::Handshaking);

        // Cannot encrypt before established
        assert!(channel.encrypt(b"test").await.is_err());
    }
}
