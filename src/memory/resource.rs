//! Layer 1 resource data types
//!
//! Resources are the raw data layer in the memory hierarchy. Every inbound
//! message or file passes through the Privacy Gate, gets classified, and is
//! stored as a Resource with its sensitivity level and routing decision.

use crate::config::SensitivityLevel;
use crate::error::{Error, Result};
use crate::privacy::ClassificationResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

/// A resource in the memory system (Layer 1).
///
/// Represents a classified piece of content with its sensitivity level,
/// storage location routing, and associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Unique resource identifier
    pub id: Uuid,
    /// User who submitted this resource
    pub user_id: String,
    /// Channel the resource arrived on
    pub channel_id: String,
    /// Chat within the channel
    pub chat_id: String,
    /// Type of content
    pub content_type: ContentType,
    /// Raw binary content
    pub raw_content: Vec<u8>,
    /// Extracted text (if applicable)
    pub text_content: Option<String>,
    /// Classification sensitivity level
    pub sensitivity: SensitivityLevel,
    /// Full classification result from the Privacy Gate
    #[serde(skip)]
    pub classification: Option<ClassificationResult>,
    /// Where this resource should be stored
    pub storage_location: StorageLocation,
    /// Taint labels from input classification (taint IDs).
    /// Propagated to Artifacts derived from this Resource.
    pub taint_labels: HashSet<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Arbitrary metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Content type of a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// Plain text message
    Text,
    /// Image file
    Image,
    /// Audio file
    Audio,
    /// Video file
    Video,
    /// Document file
    Document,
    /// Source code
    Code,
    /// Output from a tool invocation
    ToolOutput,
}

/// Storage location for a resource, determined by the Privacy Gate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StorageLocation {
    /// Store on local filesystem
    Local {
        /// Filesystem path
        path: PathBuf,
    },
    /// Store in TEE environment
    Tee {
        /// Reference identifier for the TEE store
        tee_ref: String,
    },
    /// Store in memory only (for testing or ephemeral data)
    Memory,
}

/// Builder for constructing `Resource` instances
pub struct ResourceBuilder {
    user_id: Option<String>,
    channel_id: Option<String>,
    chat_id: Option<String>,
    content_type: ContentType,
    raw_content: Vec<u8>,
    text_content: Option<String>,
    sensitivity: SensitivityLevel,
    classification: Option<ClassificationResult>,
    storage_location: StorageLocation,
    taint_labels: HashSet<String>,
    metadata: HashMap<String, serde_json::Value>,
}

impl ResourceBuilder {
    /// Create a new builder with required content type
    pub fn new(content_type: ContentType) -> Self {
        Self {
            user_id: None,
            channel_id: None,
            chat_id: None,
            content_type,
            raw_content: Vec::new(),
            text_content: None,
            sensitivity: SensitivityLevel::Normal,
            classification: None,
            storage_location: StorageLocation::Memory,
            taint_labels: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set channel ID
    pub fn channel_id(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }

    /// Set chat ID
    pub fn chat_id(mut self, chat_id: impl Into<String>) -> Self {
        self.chat_id = Some(chat_id.into());
        self
    }

    /// Set raw binary content
    pub fn raw_content(mut self, content: Vec<u8>) -> Self {
        self.raw_content = content;
        self
    }

    /// Set text content
    pub fn text_content(mut self, text: impl Into<String>) -> Self {
        self.text_content = Some(text.into());
        self
    }

    /// Set sensitivity level
    pub fn sensitivity(mut self, level: SensitivityLevel) -> Self {
        self.sensitivity = level;
        self
    }

    /// Set classification result
    pub fn classification(mut self, result: ClassificationResult) -> Self {
        self.classification = Some(result);
        self
    }

    /// Set storage location
    pub fn storage_location(mut self, location: StorageLocation) -> Self {
        self.storage_location = location;
        self
    }

    /// Add a metadata entry
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add a taint label
    pub fn taint_label(mut self, label: impl Into<String>) -> Self {
        self.taint_labels.insert(label.into());
        self
    }

    /// Set taint labels from an iterator
    pub fn taint_labels(mut self, labels: impl IntoIterator<Item = String>) -> Self {
        self.taint_labels.extend(labels);
        self
    }

    /// Build the resource, returning an error if required fields are missing
    pub fn build(self) -> Result<Resource> {
        let user_id = self
            .user_id
            .ok_or_else(|| Error::Memory("user_id is required".to_string()))?;
        let channel_id = self
            .channel_id
            .ok_or_else(|| Error::Memory("channel_id is required".to_string()))?;
        let chat_id = self
            .chat_id
            .ok_or_else(|| Error::Memory("chat_id is required".to_string()))?;

        Ok(Resource {
            id: Uuid::new_v4(),
            user_id,
            channel_id,
            chat_id,
            content_type: self.content_type,
            raw_content: self.raw_content,
            text_content: self.text_content,
            sensitivity: self.sensitivity,
            classification: self.classification,
            storage_location: self.storage_location,
            taint_labels: self.taint_labels,
            created_at: Utc::now(),
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_builder() {
        let resource = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-42")
            .raw_content(b"hello world".to_vec())
            .text_content("hello world")
            .sensitivity(SensitivityLevel::Sensitive)
            .storage_location(StorageLocation::Tee {
                tee_ref: "tee-abc".to_string(),
            })
            .metadata("source", serde_json::Value::String("test".to_string()))
            .build()
            .unwrap();

        assert_eq!(resource.user_id, "user-1");
        assert_eq!(resource.channel_id, "telegram");
        assert_eq!(resource.chat_id, "chat-42");
        assert_eq!(resource.content_type, ContentType::Text);
        assert_eq!(resource.raw_content, b"hello world");
        assert_eq!(resource.text_content.as_deref(), Some("hello world"));
        assert_eq!(resource.sensitivity, SensitivityLevel::Sensitive);
        assert_eq!(
            resource.storage_location,
            StorageLocation::Tee {
                tee_ref: "tee-abc".to_string()
            }
        );
        assert_eq!(resource.metadata.len(), 1);
    }

    #[test]
    fn test_resource_builder_missing_user_id() {
        let result = ResourceBuilder::new(ContentType::Text)
            .channel_id("telegram")
            .chat_id("chat-42")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_resource_builder_missing_channel_id() {
        let result = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .chat_id("chat-42")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_resource_builder_missing_chat_id() {
        let result = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_content_type_serialization() {
        let types = vec![
            ContentType::Text,
            ContentType::Image,
            ContentType::Audio,
            ContentType::Video,
            ContentType::Document,
            ContentType::Code,
            ContentType::ToolOutput,
        ];

        for ct in types {
            let json = serde_json::to_string(&ct).unwrap();
            let deserialized: ContentType = serde_json::from_str(&json).unwrap();
            assert_eq!(ct, deserialized);
        }
    }

    #[test]
    fn test_storage_location_variants() {
        let local = StorageLocation::Local {
            path: PathBuf::from("/tmp/data"),
        };
        let json = serde_json::to_string(&local).unwrap();
        let deserialized: StorageLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(local, deserialized);

        let tee = StorageLocation::Tee {
            tee_ref: "tee-ref-123".to_string(),
        };
        let json = serde_json::to_string(&tee).unwrap();
        let deserialized: StorageLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(tee, deserialized);

        let memory = StorageLocation::Memory;
        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: StorageLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(memory, deserialized);
    }

    #[test]
    fn test_resource_builder_taint_labels() {
        let resource = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-1")
            .taint_label("pii:email")
            .taint_label("pii:phone")
            .build()
            .unwrap();

        assert_eq!(resource.taint_labels.len(), 2);
        assert!(resource.taint_labels.contains("pii:email"));
        assert!(resource.taint_labels.contains("pii:phone"));
    }

    #[test]
    fn test_resource_builder_taint_labels_from_iter() {
        let labels = vec!["pii:ssn".to_string(), "pii:address".to_string()];
        let resource = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-1")
            .taint_labels(labels)
            .build()
            .unwrap();

        assert_eq!(resource.taint_labels.len(), 2);
        assert!(resource.taint_labels.contains("pii:ssn"));
        assert!(resource.taint_labels.contains("pii:address"));
    }

    #[test]
    fn test_resource_builder_default_empty_taint() {
        let resource = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-1")
            .build()
            .unwrap();

        assert!(resource.taint_labels.is_empty());
    }
}
