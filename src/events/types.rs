//! Event types for the Events API
//!
//! Defines wire types for event items, category counts, and subscription
//! configuration. All types use camelCase JSON serialization per the API spec.

use serde::{Deserialize, Serialize};

/// Event category
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Market,
    News,
    Social,
    Task,
    System,
    Compliance,
}

impl std::fmt::Display for EventCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Market => write!(f, "market"),
            Self::News => write!(f, "news"),
            Self::Social => write!(f, "social"),
            Self::Task => write!(f, "task"),
            Self::System => write!(f, "system"),
            Self::Compliance => write!(f, "compliance"),
        }
    }
}

impl std::str::FromStr for EventCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "market" => Ok(Self::Market),
            "news" => Ok(Self::News),
            "social" => Ok(Self::Social),
            "task" => Ok(Self::Task),
            "system" => Ok(Self::System),
            "compliance" => Ok(Self::Compliance),
            other => Err(format!("unknown event category: {}", other)),
        }
    }
}

/// A single event item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventItem {
    pub id: String,
    pub category: EventCategory,
    pub topic: String,
    pub summary: String,
    pub detail: String,
    pub timestamp: u64,
    pub source: String,
    pub subscribers: Vec<String>,
    pub reacted: bool,
    pub reacted_agent: Option<String>,
}

/// Request body for creating an event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub category: EventCategory,
    pub topic: String,
    pub summary: String,
    pub detail: String,
    pub source: String,
    #[serde(default)]
    pub subscribers: Vec<String>,
}

/// Event counts by category
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventCounts {
    pub market: u64,
    pub news: u64,
    pub social: u64,
    pub task: u64,
    pub system: u64,
    pub compliance: u64,
    pub total: u64,
}

/// Subscription configuration for a persona
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSubscription {
    pub persona_id: String,
    pub categories: Vec<EventCategory>,
}

/// Request body for updating subscriptions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionRequest {
    pub categories: Vec<EventCategory>,
}

/// Paginated response envelope
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

/// Pagination metadata
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
    pub total: u64,
    pub total_pages: u64,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: ApiErrorDetail,
}

/// API error detail
#[derive(Debug, Serialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: message.into(),
            },
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: "BAD_REQUEST".to_string(),
                message: message.into(),
            },
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: "INTERNAL_ERROR".to_string(),
                message: message.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_item_serialization() {
        let event = EventItem {
            id: "evt-1".to_string(),
            category: EventCategory::Market,
            topic: "forex.usd_cny".to_string(),
            summary: "USD/CNY broke through 7.35".to_string(),
            detail: "Exchange rate: 7.3521 (+0.42%)".to_string(),
            timestamp: 1707753600000,
            source: "Reuters Forex".to_string(),
            subscribers: vec!["financial-analyst".to_string()],
            reacted: true,
            reacted_agent: Some("financial-analyst".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"id\":\"evt-1\""));
        assert!(json.contains("\"category\":\"market\""));
        assert!(json.contains("\"reactedAgent\":\"financial-analyst\""));

        // Round-trip
        let parsed: EventItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "evt-1");
        assert_eq!(parsed.category, EventCategory::Market);
        assert!(parsed.reacted);
    }

    #[test]
    fn test_event_item_null_reacted_agent() {
        let event = EventItem {
            id: "evt-2".to_string(),
            category: EventCategory::System,
            topic: "deploy.gateway".to_string(),
            summary: "Deployed".to_string(),
            detail: "OK".to_string(),
            timestamp: 1707753600000,
            source: "CI/CD".to_string(),
            subscribers: vec![],
            reacted: false,
            reacted_agent: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"reactedAgent\":null"));
    }

    #[test]
    fn test_create_event_request_deserialization() {
        let json = r#"{
            "category": "system",
            "topic": "deploy.gateway",
            "summary": "Gateway deployed",
            "detail": "Zero-downtime update",
            "source": "CI/CD Pipeline",
            "subscribers": ["devops-engineer"]
        }"#;

        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.category, EventCategory::System);
        assert_eq!(req.topic, "deploy.gateway");
        assert_eq!(req.subscribers, vec!["devops-engineer"]);
    }

    #[test]
    fn test_create_event_request_no_subscribers() {
        let json = r#"{
            "category": "market",
            "topic": "forex",
            "summary": "Rate change",
            "detail": "Details",
            "source": "Reuters"
        }"#;

        let req: CreateEventRequest = serde_json::from_str(json).unwrap();
        assert!(req.subscribers.is_empty());
    }

    #[test]
    fn test_event_category_display() {
        assert_eq!(EventCategory::Market.to_string(), "market");
        assert_eq!(EventCategory::Compliance.to_string(), "compliance");
    }

    #[test]
    fn test_event_category_from_str() {
        assert_eq!("market".parse::<EventCategory>().unwrap(), EventCategory::Market);
        assert_eq!("compliance".parse::<EventCategory>().unwrap(), EventCategory::Compliance);
        assert!("unknown".parse::<EventCategory>().is_err());
    }

    #[test]
    fn test_event_counts_default() {
        let counts = EventCounts::default();
        assert_eq!(counts.total, 0);
        assert_eq!(counts.market, 0);
    }

    #[test]
    fn test_event_counts_serialization() {
        let counts = EventCounts {
            market: 24,
            news: 18,
            social: 12,
            task: 31,
            system: 8,
            compliance: 5,
            total: 98,
        };

        let json = serde_json::to_string(&counts).unwrap();
        assert!(json.contains("\"market\":24"));
        assert!(json.contains("\"total\":98"));
    }

    #[test]
    fn test_subscription_serialization() {
        let sub = EventSubscription {
            persona_id: "financial-analyst".to_string(),
            categories: vec![EventCategory::Market, EventCategory::Compliance],
        };

        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"personaId\":\"financial-analyst\""));
        assert!(json.contains("\"market\""));
        assert!(json.contains("\"compliance\""));
    }

    #[test]
    fn test_update_subscription_request() {
        let json = r#"{"categories": ["market", "system"]}"#;
        let req: UpdateSubscriptionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.categories.len(), 2);
        assert_eq!(req.categories[0], EventCategory::Market);
        assert_eq!(req.categories[1], EventCategory::System);
    }

    #[test]
    fn test_paginated_response() {
        let resp = PaginatedResponse {
            data: vec!["a".to_string(), "b".to_string()],
            pagination: Pagination {
                page: 1,
                per_page: 20,
                total: 2,
                total_pages: 1,
            },
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"data\":[\"a\",\"b\"]"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"perPage\":20"));
        assert!(json.contains("\"totalPages\":1"));
    }

    #[test]
    fn test_api_error_not_found() {
        let err = ApiError::not_found("Event evt-999 not found");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"NOT_FOUND\""));
        assert!(json.contains("evt-999"));
    }

    #[test]
    fn test_api_error_bad_request() {
        let err = ApiError::bad_request("Invalid category");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"BAD_REQUEST\""));
    }
}
