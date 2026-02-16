# Phase 11: Architecture Correction — Implementation Plan

## Phase 11.2: TEE Self-Detection (TeeRuntime)

### Current State
- `TeeOrchestrator` boots MicroVMs (host-side, requires `a3s-box-runtime`)
- `stub.rs` provides compile-time fallback when `real-tee` feature is off
- `SessionManager` depends on `TeeOrchestrator` for TEE lifecycle

### Target State
- `TeeRuntime` detects TEE from inside the guest (self-detection)
- No VM boot — SafeClaw IS the guest
- Sealed storage via VCEK-derived keys
- Attestation report generation for remote verification

### Changes

1. **New: `src/tee/runtime.rs`** — `TeeRuntime` with self-detection
2. **New: `src/tee/sealed.rs`** — Sealed storage (AES-GCM + VCEK key derivation)
3. **Update: `src/tee/mod.rs`** — Replace orchestrator exports with runtime
4. **Update: `src/session/manager.rs`** — Use `TeeRuntime` instead of `TeeOrchestrator`
5. **Update: `Cargo.toml`** — Remove `a3s-box-runtime` dep, add `tee` feature flag
6. **Delete: `src/tee/stub.rs`** — Replaced by TeeRuntime's graceful degradation
7. **Delete: `src/tee/orchestrator.rs`** — Host-side VM boot no longer needed
8. **Delete: `src/tee/channel.rs`** — RA-TLS channel no longer needed (we're inside)

## Phase 11.1: a3s-code Service Client (deferred)
- Keep in-process `AgentEngine` for now
- Will be replaced when a3s-code exposes unix socket service
