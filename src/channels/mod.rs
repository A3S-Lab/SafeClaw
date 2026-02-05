//! Multi-channel message adapters
//!
//! Provides unified interface for receiving and sending messages
//! across different messaging platforms.

mod adapter;
mod message;
mod telegram;
mod webchat;

pub use adapter::{ChannelAdapter, ChannelEvent};
pub use message::{InboundMessage, OutboundMessage, MessageAttachment};
pub use telegram::TelegramAdapter;
pub use webchat::WebChatAdapter;
