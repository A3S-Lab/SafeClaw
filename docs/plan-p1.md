# P1 Implementation Plan: Events API

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
├── lib.rs              # Add `pub mod events;`
└── main.rs             # Merge events_router
```

## Implementation Order (TDD)

### Step 1: Events module
1. `events/types.rs` — data types + serde tests
2. `events/store.rs` — EventStore CRUD + tests
3. `events/handler.rs` — 5 endpoints + integration tests
4. `events/mod.rs` — re-exports

### Step 2: Wire up
1. `lib.rs` — add module declarations + re-exports
2. `main.rs` — build state, merge routers
3. Run `cargo build` + `cargo test`

## Key Design Decisions

- Pagination uses the spec's `{ data, pagination }` envelope
- Error responses use `{ error: { code, message } }` (upgrade from current inline `{ error: string }`)
- Store directory: `~/.safeclaw/events/`
- Events are append-mostly; subscriptions are per-persona config files
