//! Event store backed by a3s-event (pluggable provider)
//!
//! Uses `a3s_event::EventProvider` for event publishing and subscription
//! management. Maintains an in-memory cache for filtering, pagination, and
//! text search (features not natively supported by all providers).
//!
//! Falls back to file-based persistence when no provider is available.

use crate::events::types::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Event store backend
enum Backend {
    /// Provider-backed via a3s-event (NATS, Redis, etc.)
    Provider(Box<dyn a3s_event::EventProvider>),
    /// File-based fallback (no provider available)
    File {
        events_dir: PathBuf,
        subscriptions_dir: PathBuf,
    },
}

/// Event store with pluggable provider backend and in-memory cache
///
/// The in-memory cache provides filtering, text search, and pagination
/// on top of the provider-persisted events. Subscriptions are managed through
/// provider durable consumers when available, or file-based persistence as fallback.
pub struct EventStore {
    backend: Backend,
    /// In-memory event cache (newest first)
    events: Arc<RwLock<Vec<EventItem>>>,
    /// In-memory subscription cache
    subscriptions: Arc<RwLock<Vec<EventSubscription>>>,
}

impl EventStore {
    /// Create a new event store backed by a3s-event provider
    ///
    /// Attempts to connect to NATS. If connection fails, falls back to
    /// file-based persistence at the given directory.
    pub async fn new(base_dir: PathBuf) -> std::io::Result<Self> {
        let nats_config = nats_config_from_env();

        match a3s_event::NatsProvider::connect(nats_config).await {
            Ok(provider) => {
                tracing::info!("EventStore connected to NATS JetStream");
                let store = Self {
                    backend: Backend::Provider(Box::new(provider)),
                    events: Arc::new(RwLock::new(Vec::new())),
                    subscriptions: Arc::new(RwLock::new(Vec::new())),
                };
                // Load existing events from provider into cache
                store.sync_from_provider().await;
                // Also load any legacy file-based data
                store.load_legacy_files(&base_dir).await;
                Ok(store)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to NATS ({}), falling back to file-based store",
                    e
                );
                Self::new_file_based(base_dir).await
            }
        }
    }

    /// Create a file-based event store (fallback mode)
    async fn new_file_based(base_dir: PathBuf) -> std::io::Result<Self> {
        let events_dir = base_dir.join("events");
        let subscriptions_dir = base_dir.join("subscriptions");

        tokio::fs::create_dir_all(&events_dir).await?;
        tokio::fs::create_dir_all(&subscriptions_dir).await?;

        let store = Self {
            backend: Backend::File {
                events_dir: events_dir.clone(),
                subscriptions_dir: subscriptions_dir.clone(),
            },
            events: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        };

        store.load_from_disk(&events_dir, &subscriptions_dir).await;
        Ok(store)
    }

    /// Default base directory (~/.safeclaw/events/)
    pub fn default_dir() -> PathBuf {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".safeclaw")
            .join("events")
    }

    /// Check if the store is using a provider backend (not file-based fallback)
    pub fn is_provider_backed(&self) -> bool {
        matches!(self.backend, Backend::Provider(_))
    }

    // =========================================================================
    // Event CRUD
    // =========================================================================

    /// List events with optional filtering and pagination
    pub async fn list_events(
        &self,
        category: Option<&EventCategory>,
        query: Option<&str>,
        since: Option<u64>,
        page: u64,
        per_page: u64,
    ) -> PaginatedResponse<EventItem> {
        let events = self.events.read().await;

        let filtered: Vec<&EventItem> = events
            .iter()
            .filter(|e| {
                if let Some(cat) = category {
                    if &e.category != cat {
                        return false;
                    }
                }
                if let Some(q) = query {
                    let q_lower = q.to_lowercase();
                    if !e.summary.to_lowercase().contains(&q_lower)
                        && !e.detail.to_lowercase().contains(&q_lower)
                    {
                        return false;
                    }
                }
                if let Some(ts) = since {
                    if e.timestamp < ts {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total = filtered.len() as u64;
        let total_pages = if total == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };

        let start = ((page - 1) * per_page) as usize;
        let data: Vec<EventItem> = filtered
            .into_iter()
            .skip(start)
            .take(per_page as usize)
            .cloned()
            .collect();

        PaginatedResponse {
            data,
            pagination: Pagination {
                page,
                per_page,
                total,
                total_pages,
            },
        }
    }

    /// Get a single event by ID
    pub async fn get_event(&self, id: &str) -> Option<EventItem> {
        let events = self.events.read().await;
        events.iter().find(|e| e.id == id).cloned()
    }

    /// Create a new event (id and timestamp are server-generated)
    pub async fn create_event(&self, req: CreateEventRequest) -> EventItem {
        let event = EventItem {
            id: format!("evt-{}", uuid::Uuid::new_v4()),
            category: req.category,
            topic: req.topic.clone(),
            summary: req.summary.clone(),
            detail: req.detail.clone(),
            timestamp: now_millis(),
            source: req.source.clone(),
            subscribers: req.subscribers,
            reacted: false,
            reacted_agent: None,
        };

        // Publish to provider if available
        if let Backend::Provider(ref provider) = self.backend {
            let category_str = event.category.to_string();
            let nats_event = a3s_event::Event::new(
                provider.build_subject(&category_str, &req.topic),
                &category_str,
                &req.summary,
                &req.source,
                serde_json::json!({
                    "detail": req.detail,
                    "eventId": event.id,
                }),
            )
            .with_metadata("event_id", &event.id);

            if let Err(e) = provider.publish(&nats_event).await {
                tracing::warn!(event_id = %event.id, error = %e, "Failed to publish event to provider");
            }
        }

        // Update in-memory cache
        {
            let mut events = self.events.write().await;
            events.insert(0, event.clone()); // newest first
        }

        // Persist to file (fallback or backup)
        self.persist_event(&event);
        event
    }

    /// Get event counts by category, optionally filtered by timestamp
    pub async fn counts(&self, since: Option<u64>) -> EventCounts {
        let events = self.events.read().await;
        let mut counts = EventCounts::default();

        for event in events.iter() {
            if let Some(ts) = since {
                if event.timestamp < ts {
                    continue;
                }
            }
            match event.category {
                EventCategory::Market => counts.market += 1,
                EventCategory::News => counts.news += 1,
                EventCategory::Social => counts.social += 1,
                EventCategory::Task => counts.task += 1,
                EventCategory::System => counts.system += 1,
                EventCategory::Compliance => counts.compliance += 1,
            }
            counts.total += 1;
        }

        counts
    }

    // =========================================================================
    // Subscriptions
    // =========================================================================

    /// Get subscription for a persona
    pub async fn get_subscription(&self, persona_id: &str) -> Option<EventSubscription> {
        let subs = self.subscriptions.read().await;
        subs.iter().find(|s| s.persona_id == persona_id).cloned()
    }

    /// Update subscription for a persona (upsert)
    ///
    /// When a provider is available, creates durable consumers for each subscribed
    /// category so the persona receives real-time event delivery.
    pub async fn update_subscription(
        &self,
        persona_id: &str,
        categories: Vec<EventCategory>,
    ) -> EventSubscription {
        let sub = EventSubscription {
            persona_id: persona_id.to_string(),
            categories: categories.clone(),
        };

        // Register durable consumers via provider
        if let Backend::Provider(ref provider) = self.backend {
            for cat in &categories {
                let subject = provider.category_subject(&cat.to_string());
                let consumer_name = format!("{}-{}", persona_id, cat);
                if let Err(e) = provider.subscribe_durable(&consumer_name, &subject).await {
                    tracing::warn!(
                        persona = persona_id,
                        category = %cat,
                        error = %e,
                        "Failed to register durable consumer"
                    );
                }
            }
        }

        // Update in-memory cache
        {
            let mut subs = self.subscriptions.write().await;
            if let Some(existing) = subs.iter_mut().find(|s| s.persona_id == persona_id) {
                existing.categories = sub.categories.clone();
            } else {
                subs.push(sub.clone());
            }
        }

        self.persist_subscription(&sub);
        sub
    }

    // =========================================================================
    // NATS sync
    // =========================================================================

    /// Sync events from provider into the in-memory cache
    async fn sync_from_provider(&self) {
        if let Backend::Provider(ref provider) = self.backend {
            match provider.history(None, 10_000).await {
                Ok(nats_events) => {
                    let mut events = self.events.write().await;
                    for ne in nats_events {
                        // Skip if already in cache
                        let event_id = ne
                            .metadata
                            .get("event_id")
                            .cloned()
                            .unwrap_or_else(|| ne.id.clone());
                        if events.iter().any(|e| e.id == event_id) {
                            continue;
                        }

                        let detail = ne
                            .payload
                            .get("detail")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let item = EventItem {
                            id: event_id,
                            category: ne.category.parse().unwrap_or(EventCategory::System),
                            topic: ne.subject.clone(),
                            summary: ne.summary.clone(),
                            detail,
                            timestamp: ne.timestamp,
                            source: ne.source.clone(),
                            subscribers: vec![],
                            reacted: false,
                            reacted_agent: None,
                        };
                        events.push(item);
                    }
                    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    tracing::info!(count = events.len(), "Synced events from provider");
                }
                Err(e) => {
                    tracing::warn!("Failed to sync events from provider: {}", e);
                }
            }
        }
    }

    // =========================================================================
    // File-based persistence (fallback + backup)
    // =========================================================================

    /// Load legacy file-based events into cache (for migration)
    async fn load_legacy_files(&self, base_dir: &Path) {
        let events_dir = base_dir.join("events");
        let subscriptions_dir = base_dir.join("subscriptions");
        if events_dir.exists() || subscriptions_dir.exists() {
            self.load_from_disk(&events_dir, &subscriptions_dir).await;
        }
    }

    /// Load all events and subscriptions from disk
    async fn load_from_disk(&self, events_dir: &Path, subscriptions_dir: &Path) {
        let file_events = Self::load_json_files::<EventItem>(events_dir);
        let subs = Self::load_json_files::<EventSubscription>(subscriptions_dir);

        {
            let mut events = self.events.write().await;
            for fe in file_events {
                if !events.iter().any(|e| e.id == fe.id) {
                    events.push(fe);
                }
            }
            events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        }

        *self.subscriptions.write().await = subs;
    }

    /// Load all JSON files from a directory into a Vec
    fn load_json_files<T: serde::de::DeserializeOwned>(dir: &Path) -> Vec<T> {
        let mut items = Vec::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("Failed to read directory {}: {}", dir.display(), e);
                }
                return items;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str(&data) {
                    Ok(item) => items.push(item),
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {}", path.display(), e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", path.display(), e);
                }
            }
        }

        items
    }

    /// Persist a single event to disk (fire-and-forget)
    fn persist_event(&self, event: &EventItem) {
        let dir = match &self.backend {
            Backend::File { events_dir, .. } => events_dir.clone(),
            Backend::Provider(_) => Self::default_dir().join("events"),
        };
        let event = event.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::create_dir_all(&dir).await;
            let path = dir.join(format!("{}.json", event.id));
            match serde_json::to_string_pretty(&event) {
                Ok(json) => {
                    if let Err(e) = tokio::fs::write(&path, json).await {
                        tracing::warn!("Failed to persist event {}: {}", event.id, e);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to serialize event {}: {}", event.id, e);
                }
            }
        });
    }

    /// Persist a subscription to disk (fire-and-forget)
    fn persist_subscription(&self, sub: &EventSubscription) {
        let dir = match &self.backend {
            Backend::File {
                subscriptions_dir, ..
            } => subscriptions_dir.clone(),
            Backend::Provider(_) => Self::default_dir().join("subscriptions"),
        };
        let sub = sub.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::create_dir_all(&dir).await;
            let path = dir.join(format!("{}.json", sub.persona_id));
            match serde_json::to_string_pretty(&sub) {
                Ok(json) => {
                    if let Err(e) = tokio::fs::write(&path, json).await {
                        tracing::warn!(
                            "Failed to persist subscription {}: {}",
                            sub.persona_id,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to serialize subscription {}: {}",
                        sub.persona_id,
                        e
                    );
                }
            }
        });
    }
}

/// Build NATS config from environment variables
fn nats_config_from_env() -> a3s_event::NatsConfig {
    let mut config = a3s_event::NatsConfig::default();

    if let Ok(url) = std::env::var("NATS_URL") {
        config.url = url;
    }
    if let Ok(token) = std::env::var("NATS_TOKEN") {
        config.token = Some(token);
    }
    if let Ok(creds) = std::env::var("NATS_CREDENTIALS") {
        config.credentials_path = Some(creds);
    }

    config
}

/// Current time in Unix milliseconds
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Tests use file-based fallback (no NATS server required)
    async fn make_store() -> (EventStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = EventStore::new_file_based(dir.path().to_path_buf())
            .await
            .unwrap();
        (store, dir)
    }

    fn make_create_request(category: EventCategory, topic: &str) -> CreateEventRequest {
        CreateEventRequest {
            category,
            topic: topic.to_string(),
            summary: format!("Summary for {}", topic),
            detail: format!("Detail for {}", topic),
            source: "test".to_string(),
            subscribers: vec!["analyst".to_string()],
        }
    }

    #[tokio::test]
    async fn test_create_and_get_event() {
        let (store, _dir) = make_store().await;

        let req = make_create_request(EventCategory::Market, "forex.usd_cny");
        let event = store.create_event(req).await;

        assert!(event.id.starts_with("evt-"));
        assert_eq!(event.category, EventCategory::Market);
        assert_eq!(event.topic, "forex.usd_cny");
        assert!(!event.reacted);
        assert!(event.reacted_agent.is_none());
        assert!(event.timestamp > 0);

        // Get by ID
        let fetched = store.get_event(&event.id).await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, event.id);
    }

    #[tokio::test]
    async fn test_get_event_not_found() {
        let (store, _dir) = make_store().await;
        assert!(store.get_event("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let (store, _dir) = make_store().await;
        let result = store.list_events(None, None, None, 1, 20).await;
        assert!(result.data.is_empty());
        assert_eq!(result.pagination.total, 0);
        assert_eq!(result.pagination.total_pages, 0);
    }

    #[tokio::test]
    async fn test_list_events_pagination() {
        let (store, _dir) = make_store().await;

        // Create 5 events
        for i in 0..5 {
            let req = make_create_request(EventCategory::Market, &format!("topic-{}", i));
            store.create_event(req).await;
        }

        // Page 1, 2 per page
        let result = store.list_events(None, None, None, 1, 2).await;
        assert_eq!(result.data.len(), 2);
        assert_eq!(result.pagination.total, 5);
        assert_eq!(result.pagination.total_pages, 3);
        assert_eq!(result.pagination.page, 1);

        // Page 3 (last page, 1 item)
        let result = store.list_events(None, None, None, 3, 2).await;
        assert_eq!(result.data.len(), 1);
    }

    #[tokio::test]
    async fn test_list_events_filter_by_category() {
        let (store, _dir) = make_store().await;

        store
            .create_event(make_create_request(EventCategory::Market, "forex"))
            .await;
        store
            .create_event(make_create_request(EventCategory::System, "deploy"))
            .await;
        store
            .create_event(make_create_request(EventCategory::Market, "stock"))
            .await;

        let result = store
            .list_events(Some(&EventCategory::Market), None, None, 1, 20)
            .await;
        assert_eq!(result.data.len(), 2);
        assert!(result
            .data
            .iter()
            .all(|e| e.category == EventCategory::Market));
    }

    #[tokio::test]
    async fn test_list_events_search() {
        let (store, _dir) = make_store().await;

        store
            .create_event(make_create_request(EventCategory::Market, "forex"))
            .await;
        store
            .create_event(make_create_request(EventCategory::News, "politics"))
            .await;

        // Search matches summary "Summary for forex"
        let result = store.list_events(None, Some("forex"), None, 1, 20).await;
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].topic, "forex");
    }

    #[tokio::test]
    async fn test_list_events_since() {
        let (store, _dir) = make_store().await;

        store
            .create_event(make_create_request(EventCategory::Market, "old"))
            .await;

        // All events created just now should be after timestamp 0
        let result = store.list_events(None, None, Some(0), 1, 20).await;
        assert_eq!(result.data.len(), 1);

        // No events after far-future timestamp
        let result = store
            .list_events(None, None, Some(u64::MAX), 1, 20)
            .await;
        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn test_counts() {
        let (store, _dir) = make_store().await;

        store
            .create_event(make_create_request(EventCategory::Market, "a"))
            .await;
        store
            .create_event(make_create_request(EventCategory::Market, "b"))
            .await;
        store
            .create_event(make_create_request(EventCategory::System, "c"))
            .await;

        let counts = store.counts(None).await;
        assert_eq!(counts.market, 2);
        assert_eq!(counts.system, 1);
        assert_eq!(counts.news, 0);
        assert_eq!(counts.total, 3);
    }

    #[tokio::test]
    async fn test_counts_with_since() {
        let (store, _dir) = make_store().await;

        store
            .create_event(make_create_request(EventCategory::Market, "a"))
            .await;

        let counts = store.counts(Some(u64::MAX)).await;
        assert_eq!(counts.total, 0);
    }

    #[tokio::test]
    async fn test_subscription_crud() {
        let (store, _dir) = make_store().await;

        // No subscription initially
        assert!(store.get_subscription("analyst").await.is_none());

        // Create
        let sub = store
            .update_subscription(
                "analyst",
                vec![EventCategory::Market, EventCategory::Compliance],
            )
            .await;
        assert_eq!(sub.persona_id, "analyst");
        assert_eq!(sub.categories.len(), 2);

        // Read
        let fetched = store.get_subscription("analyst").await.unwrap();
        assert_eq!(fetched.categories.len(), 2);

        // Update (overwrite)
        let updated = store
            .update_subscription("analyst", vec![EventCategory::System])
            .await;
        assert_eq!(updated.categories.len(), 1);
        assert_eq!(updated.categories[0], EventCategory::System);
    }

    #[tokio::test]
    async fn test_persistence_round_trip() {
        let dir = TempDir::new().unwrap();

        // Create store, add data
        {
            let store = EventStore::new_file_based(dir.path().to_path_buf())
                .await
                .unwrap();
            store
                .create_event(make_create_request(EventCategory::Market, "forex"))
                .await;
            store
                .update_subscription("analyst", vec![EventCategory::Market])
                .await;

            // Wait for async persistence
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Reload from disk
        let store = EventStore::new_file_based(dir.path().to_path_buf())
            .await
            .unwrap();
        let events = store.list_events(None, None, None, 1, 20).await;
        assert_eq!(events.data.len(), 1);
        assert_eq!(events.data[0].topic, "forex");

        let sub = store.get_subscription("analyst").await;
        assert!(sub.is_some());
        assert_eq!(sub.unwrap().categories[0], EventCategory::Market);
    }

    #[tokio::test]
    async fn test_load_skips_corrupt_files() {
        let dir = TempDir::new().unwrap();
        let events_dir = dir.path().join("events");
        std::fs::create_dir_all(&events_dir).unwrap();

        // Write a corrupt file
        std::fs::write(events_dir.join("bad.json"), "not valid json").unwrap();

        // Should not panic, just skip
        let store = EventStore::new_file_based(dir.path().to_path_buf())
            .await
            .unwrap();
        let events = store.list_events(None, None, None, 1, 20).await;
        assert!(events.data.is_empty());
    }

    #[tokio::test]
    async fn test_fallback_to_file_when_nats_unavailable() {
        // With no NATS server running, new() should fall back to file-based
        let dir = TempDir::new().unwrap();
        let store = EventStore::new(dir.path().to_path_buf()).await.unwrap();
        assert!(!store.is_provider_backed());

        // Should still work normally
        let event = store
            .create_event(make_create_request(EventCategory::Market, "test"))
            .await;
        assert!(event.id.starts_with("evt-"));
    }
}
