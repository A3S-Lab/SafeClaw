# P1 Implementation Plan: Knowledge Base + Events API

## Architecture

Follow the existing `agent/` module pattern exactly:
- Each domain gets its own module directory with `mod.rs`, `handler.rs`, `store.rs`, `types.rs`
- File-based JSON persistence (same pattern as `AgentSessionStore`)
- Router function merged into the app in `main.rs`
- State injected via axum `State` extractor

## File Structure

```
crates/safeclaw/src/
├── events/
│   ├── mod.rs          # Re-exports
│   ├── types.rs        # EventItem, EventCounts, EventSubscription
│   ├── store.rs        # EventStore (file-based JSON persistence)
│   └── handler.rs      # 5 REST endpoints + events_router()
├── knowledge/
│   ├── mod.rs          # Re-exports
│   ├── types.rs        # KnowledgeItem, KnowledgeUsage
│   ├── store.rs        # KnowledgeStore (file-based JSON + filesystem)
│   └── handler.rs      # 8 REST endpoints + knowledge_router()
├── lib.rs              # Add `pub mod events; pub mod knowledge;`
└── main.rs             # Merge events_router + knowledge_router
```

## Implementation Order (TDD)

### Step 1: Events module
1. `events/types.rs` — data types + serde tests
2. `events/store.rs` — EventStore CRUD + tests
3. `events/handler.rs` — 5 endpoints + integration tests
4. `events/mod.rs` — re-exports

### Step 2: Knowledge module
1. `knowledge/types.rs` — data types + serde tests
2. `knowledge/store.rs` — KnowledgeStore CRUD + tests
3. `knowledge/handler.rs` — 8 endpoints + integration tests
4. `knowledge/mod.rs` — re-exports

### Step 3: Wire up
1. `lib.rs` — add module declarations + re-exports
2. `main.rs` — build state, merge routers
3. Run `cargo build` + `cargo test`

## Key Design Decisions

- Pagination uses the spec's `{ data, pagination }` envelope
- Error responses use `{ error: { code, message } }` (upgrade from current inline `{ error: string }`)
- Store directories: `~/.safeclaw/events/`, `~/.safeclaw/knowledge/`
- Knowledge files stored on filesystem; metadata in JSON index
- Events are append-mostly; subscriptions are per-persona config files
- No multipart upload in first pass — knowledge file upload uses `multipart/form-data` via axum's `Multipart` extractor
