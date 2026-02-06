//! Multi-channel message adapters
//!
//! Provides unified interface for receiving and sending messages
//! across different messaging platforms.

mod adapter;
mod dingtalk;
mod discord;
mod feishu;
mod message;
mod slack;
mod telegram;
mod webchat;
mod wecom;

pub use adapter::{ChannelAdapter, ChannelEvent};
pub use dingtalk::DingTalkAdapter;
pub use discord::DiscordAdapter;
pub use feishu::FeishuAdapter;
pub use message::{InboundMessage, MessageAttachment, OutboundMessage};
pub use slack::SlackAdapter;
pub use telegram::TelegramAdapter;
pub use webchat::WebChatAdapter;
pub use wecom::WeComAdapter;
