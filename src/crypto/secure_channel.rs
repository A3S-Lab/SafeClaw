//! Secure channel for encrypted communication with TEE
//!
//! **Threat model**: Defends against A3 (network attacker) and A5 (insider) at AS-5.
//! See `docs/threat-model.md` §4 AS-5, §5.

use super::{decrypt, encrypt, KEY_SIZE};
use crate::crypto::keys::{EphemeralKeyPair, PublicKey, SharedSecret};
use crate::error::{Error, Result};
use hkdf::Hkdf;
use sha2::Sha256;
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Protocol version bound into HKDF info string to prevent cross-version key reuse.
const PROTOCOL_VERSION: &str = "safeclaw-session-v1";

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

/// Session key wrapper with secure erasure on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
struct SessionKey([u8; KEY_SIZE]);

/// Secure channel for encrypted communication.
///
/// Uses ephemeral X25519 key exchange with HKDF-SHA256 key derivation
/// for forward secrecy. Session keys are zeroized on drop.
pub struct SecureChannel {
    state: Arc<RwLock<ChannelState>>,
    session_key: Arc<RwLock<Option<SessionKey>>>,
    local_ephemeral: Arc<RwLock<Option<EphemeralKeyPair>>>,
    local_public_bytes: [u8; 32],
    channel_id: String,
}

impl SecureChannel {
    /// Create a new secure channel with fresh ephemeral keys
    pub fn new(channel_id: String) -> Self {
        let ephemeral = EphemeralKeyPair::generate();
        let local_public_bytes = *ephemeral.public_key().as_bytes();

        Self {
            state: Arc::new(RwLock::new(ChannelState::Initial)),
            session_key: Arc::new(RwLock::new(None)),
            local_ephemeral: Arc::new(RwLock::new(Some(ephemeral))),
            local_public_bytes,
            channel_id,
        }
    }

    /// Get the channel ID
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    /// Get the local public key for handshake
    pub fn local_public_key(&self) -> PublicKey {
        PublicKey::from_bytes(&self.local_public_bytes)
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
            public_key: self.local_public_bytes.to_vec(),
        })
    }

    /// Complete handshake with remote public key.
    ///
    /// Performs X25519 Diffie-Hellman, then derives the session key using
    /// HKDF-SHA256 (RFC 5869). The ephemeral secret is consumed and cannot
    /// be reused (forward secrecy).
    pub async fn complete_handshake(&self, remote_public_bytes: &[u8; 32]) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != ChannelState::Handshaking {
            return Err(Error::Crypto(
                "Channel not in handshaking state".to_string(),
            ));
        }

        // Take the ephemeral key pair (consumed by DH)
        let ephemeral = self
            .local_ephemeral
            .write()
            .await
            .take()
            .ok_or_else(|| Error::Crypto("Ephemeral key already consumed".to_string()))?;

        let remote_public = PublicKey::from_bytes(remote_public_bytes);

        // Perform key exchange — ephemeral secret is destroyed after this
        let shared_secret = ephemeral
            .diffie_hellman(&remote_public)
            .map_err(|e| Error::Crypto(e.to_string()))?;

        // Derive session key using HKDF-SHA256 (RFC 5869)
        let session_key = derive_session_key(
            &shared_secret,
            &self.local_public_bytes,
            remote_public_bytes,
            &self.channel_id,
        );

        // shared_secret is ZeroizeOnDrop — dropped here

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

        encrypt(&key.0, plaintext)
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

        decrypt(&key.0, ciphertext)
    }

    /// Close the channel and zeroize the session key
    pub async fn close(&self) {
        let mut state = self.state.write().await;
        *state = ChannelState::Closed;

        // SessionKey implements ZeroizeOnDrop — replacing with None drops and zeroizes
        *self.session_key.write().await = None;
    }
}

/// Handshake initialization message
#[derive(Debug, Clone)]
pub struct HandshakeInit {
    /// Channel identifier
    pub channel_id: String,
    /// Local ephemeral public key
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

/// Derive session key from shared secret using HKDF-SHA256 (RFC 5869).
///
/// - IKM: X25519 shared secret
/// - Salt: sorted concatenation of both public keys (deterministic regardless of role)
/// - Info: protocol version + channel ID (binds key to specific session)
fn derive_session_key(
    shared_secret: &SharedSecret,
    local_pub: &[u8; 32],
    remote_pub: &[u8; 32],
    channel_id: &str,
) -> SessionKey {
    // Sort public keys so both sides derive the same salt regardless of role
    let salt = if local_pub < remote_pub {
        [local_pub.as_slice(), remote_pub.as_slice()].concat()
    } else {
        [remote_pub.as_slice(), local_pub.as_slice()].concat()
    };

    let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared_secret.as_bytes());

    // Info string binds key to protocol version and channel
    let info = format!("{}:{}", PROTOCOL_VERSION, channel_id);

    let mut key = [0u8; KEY_SIZE];
    hkdf.expand(info.as_bytes(), &mut key)
        .expect("HKDF expand failed — invalid output length");

    SessionKey(key)
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

    #[tokio::test]
    async fn test_channel_close_clears_key() {
        let channel1 = SecureChannel::new("test".to_string());
        let channel2 = SecureChannel::new("test".to_string());

        let init1 = channel1.start_handshake().await.unwrap();
        let init2 = channel2.start_handshake().await.unwrap();

        let pk1: [u8; 32] = init1.public_key.try_into().unwrap();
        let pk2: [u8; 32] = init2.public_key.try_into().unwrap();

        channel1.complete_handshake(&pk2).await.unwrap();
        channel2.complete_handshake(&pk1).await.unwrap();

        // Close channel
        channel1.close().await;
        assert_eq!(channel1.state().await, ChannelState::Closed);

        // Cannot encrypt after close
        assert!(channel1.encrypt(b"test").await.is_err());
    }

    #[tokio::test]
    async fn test_double_handshake_fails() {
        let channel = SecureChannel::new("test".to_string());
        channel.start_handshake().await.unwrap();

        // Second start_handshake should fail
        assert!(channel.start_handshake().await.is_err());
    }

    #[test]
    fn test_derive_session_key_deterministic() {
        let shared = SharedSecret([0xAB; 32]);
        let local = [1u8; 32];
        let remote = [2u8; 32];

        let k1 = derive_session_key(&shared, &local, &remote, "ch1");
        let k2 = derive_session_key(&shared, &local, &remote, "ch1");
        assert_eq!(k1.0, k2.0);
    }

    #[test]
    fn test_derive_session_key_role_independent() {
        // Both sides should derive the same key regardless of who is "local" vs "remote"
        let shared = SharedSecret([0xAB; 32]);
        let pub_a = [1u8; 32];
        let pub_b = [2u8; 32];

        let k_ab = derive_session_key(&shared, &pub_a, &pub_b, "ch1");
        let k_ba = derive_session_key(&shared, &pub_b, &pub_a, "ch1");
        assert_eq!(k_ab.0, k_ba.0);
    }

    #[test]
    fn test_derive_session_key_different_channels() {
        let shared = SharedSecret([0xAB; 32]);
        let local = [1u8; 32];
        let remote = [2u8; 32];

        let k1 = derive_session_key(&shared, &local, &remote, "channel-1");
        let k2 = derive_session_key(&shared, &local, &remote, "channel-2");
        assert_ne!(k1.0, k2.0);
    }
}
