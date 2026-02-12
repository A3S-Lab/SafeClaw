# SafeClaw API Specification

> Version: 0.1.0 | Status: Draft | Last Updated: 2025-07-14

## Table of Contents

- [Overview](#overview)
- [Conventions](#conventions)
- [Authentication](#authentication)
- [1. Agent Chat](#1-agent-chat)
- [2. Events](#2-events)
- [3. Knowledge Base](#3-knowledge-base)
- [4. Assets / Projects](#4-assets--projects)
- [5. Enterprise Systems](#5-enterprise-systems)
- [6. Agent Marketplace](#6-agent-marketplace)
- [7. Settings](#7-settings)
- [8. Common Endpoints](#8-common-endpoints)
- [9. WebSocket Protocol](#9-websocket-protocol)
- [Implementation Priority](#implementation-priority)

---

## Overview

SafeClaw exposes a REST + WebSocket API for its 7 UI pages. The AI agent backend is powered by **a3s-code** `SessionManager`, which provides the full agent loop: LLM conversation, tool calling, HITL (human-in-the-loop) confirmation, and session persistence.

### Architecture

```
Browser (React UI)
  │
  ├── REST  ──→  Axum Router (/api/v1/...)  ──→  Handlers  ──→  a3s-code SessionManager
  │                                                           ──→  Domain stores (Knowledge, Events, ...)
  └── WS    ──→  /ws/agent/browser/:id      ──→  AgentEngine ──→  SessionManager.generate_streaming()
                                                               ──→  AgentEvent → BrowserIncomingMessage
```

### Base URLs

| Protocol | URL |
|----------|-----|
| REST API | `http://127.0.0.1:18790/api/v1` |
| WebSocket | `ws://127.0.0.1:18790/ws/...` |
| Health (no prefix) | `http://127.0.0.1:18790/health` |

---

## Conventions

### URL & JSON Style

- URL paths: `kebab-case` (e.g., `/api/v1/my-agents`)
- JSON fields: `camelCase` (e.g., `sessionId`, `totalCost`)
- Timestamps: Unix milliseconds (`1707753600000`) unless noted otherwise

### Pagination

Query: `?page=1&perPage=20`

```json
{
  "data": [],
  "pagination": {
    "page": 1,
    "perPage": 20,
    "total": 142,
    "totalPages": 8
  }
}
```

### Filtering & Search

- Full-text search: `?q=keyword`
- Field filters: `?category=finance&status=running`
- Date range: `?since=1707753600000` (Unix ms)
- Sorting: `?sortBy=updatedAt&sortOrder=desc`

### Error Response

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Session sess-abc123 not found"
  }
}
```

Standard error codes: `BAD_REQUEST`, `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `INTERNAL_ERROR`.

### Authentication (Reserved)

```
Authorization: Bearer <token>
```

Not enforced in v0.1. All endpoints are open. Auth header is accepted and ignored.

---

## 1. Agent Chat

The agent chat system is backed by **a3s-code** `SessionManager`. Each session wraps an LLM agent loop with tool execution, permission management, and streaming output. Session CRUD is REST; real-time chat flows through WebSocket.

### Endpoints (8)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/agent/sessions` | Create session |
| GET | `/api/v1/agent/sessions` | List sessions |
| GET | `/api/v1/agent/sessions/:id` | Get session detail |
| PATCH | `/api/v1/agent/sessions/:id` | Update session |
| DELETE | `/api/v1/agent/sessions/:id` | Delete session |
| POST | `/api/v1/agent/sessions/:id/relaunch` | Relaunch session |
| GET | `/api/v1/agent/backends` | List available models |
| WS | `/ws/agent/browser/:id` | Real-time chat |

### POST `/api/v1/agent/sessions`

Create a new agent session. This calls `SessionManager::create_session()` under the hood, initializing an LLM client, tool set, and permission policy.

**Request:**

```json
{
  "model": "claude-sonnet-4-20250514",
  "permissionMode": "default",
  "cwd": "/home/user/project",
  "baseUrl": "https://api.anthropic.com",
  "apiKey": "sk-...",
  "systemPrompt": "You are a financial analyst...",
  "skills": ["code-review", "test-writer"]
}
```

All fields are optional. `model` defaults to the configured default model. `permissionMode` defaults to `"default"`. `cwd` defaults to the server working directory.

**Response:** `201 Created`

```json
{
  "sessionId": "sess-a1b2c3",
  "pid": 12345,
  "state": "connected",
  "model": "claude-sonnet-4-20250514",
  "permissionMode": "default",
  "cwd": "/home/user/project",
  "createdAt": "2025-01-15T10:30:00Z",
  "archived": false,
  "name": null
}
```

### GET `/api/v1/agent/sessions`

List all sessions. Backed by `SessionManager::list_sessions()` plus UI-layer metadata (name, archived state).

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `archived` | bool | Filter by archived status. Default: show all |

**Response:** `200 OK`

```json
[
  {
    "sessionId": "sess-a1b2c3",
    "pid": 12345,
    "state": "connected",
    "model": "claude-sonnet-4-20250514",
    "permissionMode": "default",
    "cwd": "/home/user/project",
    "createdAt": "2025-01-15T10:30:00Z",
    "cliSessionId": "cli-xyz",
    "archived": false,
    "name": "Refactor auth module"
  }
]
```

`state` is one of: `"starting"`, `"connected"`, `"running"`, `"exited"`.

### GET `/api/v1/agent/sessions/:id`

Get session detail. Returns the same shape as the list item.

**Response:** `200 OK` — `AgentProcessInfo` object (same as list item).

**Error:** `404` if session not found.

### PATCH `/api/v1/agent/sessions/:id`

Update session metadata (UI-layer only, does not affect the a3s-code session).

**Request:**

```json
{
  "name": "Auth module refactor",
  "archived": true
}
```

Both fields optional. At least one must be provided.

**Response:** `200 OK` — Updated `AgentProcessInfo`.

### DELETE `/api/v1/agent/sessions/:id`

Delete a session. Calls `SessionManager::destroy_session()` to clean up the agent loop, then removes UI-layer state.

**Response:** `204 No Content`

### POST `/api/v1/agent/sessions/:id/relaunch`

Destroy and recreate a session with the same configuration. Useful when a session enters an error state.

**Response:** `200 OK` — New `AgentProcessInfo`.

### GET `/api/v1/agent/backends`

List available model backends. Derived from `CodeConfig.providers` and their configured models.

**Response:** `200 OK`

```json
[
  {
    "id": "claude-sonnet-4-20250514",
    "name": "Claude Sonnet 4",
    "provider": "anthropic",
    "isDefault": true
  },
  {
    "id": "gpt-4o",
    "name": "GPT-4o",
    "provider": "openai",
    "isDefault": false
  }
]
```

### WS `/ws/agent/browser/:id`

WebSocket connection for real-time agent chat. See [Section 9: WebSocket Protocol](#9-websocket-protocol) for the full message schema.

Connection flow:
1. Client opens WebSocket to `/ws/agent/browser/:id`
2. Server sends `session_init` with current `AgentSessionState`
3. Server replays `message_history` if reconnecting
4. Client sends `user_message` to start generation
5. Server streams `AgentEvent`s translated to `BrowserIncomingMessage`s

---

## 2. Events

Events represent real-time signals from external sources (market data, news, social media) and internal triggers (system alerts, compliance flags, task completions). Personas can subscribe to event categories and the system routes events to relevant agents.

### Endpoints (5)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/events` | List events |
| GET | `/api/v1/events/:id` | Event detail |
| POST | `/api/v1/events` | Create event |
| GET | `/api/v1/events/counts` | Category counts |
| PUT | `/api/v1/events/subscriptions/:personaId` | Update subscriptions |

### GET `/api/v1/events`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `category` | string | `market\|news\|social\|task\|system\|compliance` |
| `q` | string | Full-text search on summary/detail |
| `since` | number | Unix ms timestamp, events after this time |
| `page` | number | Page number (default: 1) |
| `perPage` | number | Items per page (default: 20) |

**Response:** `200 OK` — Paginated `EventItem[]`

### GET `/api/v1/events/:id`

**Response:** `200 OK`

```json
{
  "id": "evt-1",
  "category": "market",
  "topic": "forex.usd_cny",
  "summary": "USD/CNY broke through 7.35",
  "detail": "Exchange rate: 7.3521 (+0.42%), triggered by Fed policy signal",
  "timestamp": 1707753600000,
  "source": "Reuters Forex",
  "subscribers": ["financial-analyst", "risk-analyst"],
  "reacted": true,
  "reactedAgent": "financial-analyst"
}
```

### POST `/api/v1/events`

Create an event (triggered by system or agent).

**Request:**

```json
{
  "category": "system",
  "topic": "deploy.gateway",
  "summary": "Gateway v3.12.1 deployed successfully",
  "detail": "Zero-downtime rolling update completed in 45s",
  "source": "CI/CD Pipeline",
  "subscribers": ["devops-engineer"]
}
```

`id` and `timestamp` are server-generated.

**Response:** `201 Created` — Full `EventItem`.

### GET `/api/v1/events/counts`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `since` | number | Unix ms timestamp |

**Response:** `200 OK`

```json
{
  "market": 24,
  "news": 18,
  "social": 12,
  "task": 31,
  "system": 8,
  "compliance": 5,
  "total": 98
}
```

### PUT `/api/v1/events/subscriptions/:personaId`

Update which event categories a persona subscribes to.

**Request:**

```json
{
  "categories": ["market", "compliance", "system"]
}
```

**Response:** `200 OK`

```json
{
  "personaId": "financial-analyst",
  "categories": ["market", "compliance", "system"]
}
```

---

## 3. Knowledge Base

Hierarchical file/folder storage for organizational documents. Supports nested folders, file upload/download, starring, tagging, and full-text search.

### Endpoints (8)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/knowledge/items` | List items |
| GET | `/api/v1/knowledge/items/:id` | Item detail |
| POST | `/api/v1/knowledge/folders` | Create folder |
| POST | `/api/v1/knowledge/files` | Upload file |
| PATCH | `/api/v1/knowledge/items/:id` | Update item |
| DELETE | `/api/v1/knowledge/items/:id` | Delete item |
| GET | `/api/v1/knowledge/files/:id/download` | Download file |
| GET | `/api/v1/knowledge/usage` | Storage usage |

### KnowledgeItem Schema

```json
{
  "id": "kb-1",
  "name": "AML_Manual_v3.2.docx",
  "type": "docx",
  "size": 2450000,
  "updatedAt": 1707753600000,
  "updatedBy": "risk-analyst",
  "starred": false,
  "tags": ["AML", "KYC"],
  "parentId": "kb-root",
  "childCount": null
}
```

`type` is one of: `folder`, `docx`, `xlsx`, `pptx`, `pdf`, `md`, `txt`, `csv`, `json`, `html`.

For folders, `size` is `null` and `childCount` is the number of direct children.

### GET `/api/v1/knowledge/items`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `parentId` | string | List children of this folder. Default: root items |
| `q` | string | Full-text search across all items |
| `starred` | bool | Filter starred items only |
| `type` | string | Filter by file type |
| `sortBy` | string | `name\|updatedAt\|size` (default: `name`) |
| `sortOrder` | string | `asc\|desc` (default: `asc`) |
| `page` | number | Page number (default: 1) |
| `perPage` | number | Items per page (default: 50) |

**Response:** `200 OK` — Paginated `KnowledgeItem[]`

When `q` is provided, search is recursive across all descendants regardless of `parentId`.

### GET `/api/v1/knowledge/items/:id`

**Response:** `200 OK` — `KnowledgeItem` with full metadata.

For folders, includes `children: KnowledgeItem[]` (one level deep).

### POST `/api/v1/knowledge/folders`

**Request:**

```json
{
  "name": "Q1 Reports",
  "parentId": "kb-finance"
}
```

`parentId` is optional; defaults to root.

**Response:** `201 Created` — `KnowledgeItem` (type: `folder`).

### POST `/api/v1/knowledge/files`

Upload a file via `multipart/form-data`.

**Form Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | binary | yes | File content |
| `parentId` | string | no | Target folder (default: root) |
| `tags` | string | no | Comma-separated tags |

**Response:** `201 Created` — `KnowledgeItem`.

### PATCH `/api/v1/knowledge/items/:id`

Update item metadata. Works for both files and folders.

**Request:**

```json
{
  "name": "AML_Manual_v4.0.docx",
  "starred": true,
  "tags": ["AML", "KYC", "2025"],
  "parentId": "kb-compliance"
}
```

All fields optional. Setting `parentId` moves the item to a different folder.

**Response:** `200 OK` — Updated `KnowledgeItem`.

### DELETE `/api/v1/knowledge/items/:id`

Delete an item. For folders, recursively deletes all descendants.

**Response:** `204 No Content`

### GET `/api/v1/knowledge/files/:id/download`

Download file content.

**Response:** `200 OK` — Binary stream with `Content-Disposition: attachment; filename="..."` header.

**Error:** `404` if not found, `400` if target is a folder.

### GET `/api/v1/knowledge/usage`

**Response:** `200 OK`

```json
{
  "totalFiles": 142,
  "totalFolders": 23,
  "totalSize": 1073741824,
  "usedPercent": 34.2,
  "quota": 3221225472,
  "byType": {
    "pdf": { "count": 45, "size": 524288000 },
    "docx": { "count": 32, "size": 268435456 },
    "xlsx": { "count": 18, "size": 134217728 }
  }
}
```

Sizes are in bytes.

---

## 4. Assets / Projects

Projects represent code repositories — both enterprise services and agent-managed projects. Enterprise projects are traditional team-owned repos; agent projects are autonomously developed by a3s-code agents with goal tracking and milestones.

### Endpoints (7)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/projects` | List projects |
| GET | `/api/v1/projects/:id` | Project detail |
| GET | `/api/v1/projects/:id/files` | File tree |
| GET | `/api/v1/projects/:id/files/content` | File content |
| POST | `/api/v1/projects` | Import repository |
| PATCH | `/api/v1/projects/:id` | Update project |
| GET | `/api/v1/projects/stats` | Project statistics |

### ProjectItem Schema

```json
{
  "id": "proj-1",
  "name": "safeclaw-gateway",
  "description": "Cross-border payment core gateway",
  "category": "enterprise",
  "status": "active",
  "language": "Rust",
  "languages": ["Rust", "SQL"],
  "git": {
    "url": "https://git.internal.com/infra/safeclaw-gateway",
    "branch": "main",
    "lastCommit": "feat: add SWIFT webhook",
    "lastCommitTime": 1707753600000,
    "commitCount": 1847,
    "openPRs": 3,
    "stars": 24
  },
  "team": ["fullstack-engineer", "devops-engineer"],
  "version": "v3.12.0",
  "tags": ["core-service", "payment"],
  "agentId": null,
  "devGoal": null,
  "devStatus": null,
  "devProgress": null,
  "milestones": []
}
```

`category`: `enterprise` | `agent`

`status`: `active` | `stable` | `archived` | `developing`

For agent projects (`category: "agent"`), the following fields are populated:
- `agentId` — the persona ID of the managing agent
- `devGoal` — the agent's current development objective
- `devStatus` — `planning` | `coding` | `testing` | `reviewing` | `deployed` | `paused`
- `devProgress` — 0-100 percentage
- `milestones` — array of `{ id, title, done }`

### GET `/api/v1/projects`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `category` | string | `enterprise\|agent` |
| `status` | string | `active\|stable\|archived\|developing` |
| `q` | string | Search name/description |
| `language` | string | Filter by primary language |
| `page` | number | Page number (default: 1) |
| `perPage` | number | Items per page (default: 20) |

**Response:** `200 OK` — Paginated `ProjectItem[]`

### GET `/api/v1/projects/:id`

**Response:** `200 OK` — Full `ProjectItem` including `milestones`.

### GET `/api/v1/projects/:id/files`

Get the file tree for a project directory.

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `path` | string | Subdirectory path (default: root `/`) |

**Response:** `200 OK`

```json
[
  {
    "name": "src",
    "type": "folder",
    "children": [
      { "name": "main.rs", "type": "file", "language": "rust" },
      { "name": "handlers", "type": "folder", "children": [] }
    ]
  },
  { "name": "Cargo.toml", "type": "file", "language": "toml" }
]
```

### GET `/api/v1/projects/:id/files/content`

Get the content of a single file.

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `path` | string | File path relative to project root (required) |

**Response:** `200 OK`

```json
{
  "path": "src/main.rs",
  "language": "rust",
  "content": "fn main() {\n    println!(\"Hello\");\n}\n",
  "size": 42
}
```

### POST `/api/v1/projects`

Import a repository as a project.

**Request:**

```json
{
  "name": "new-service",
  "description": "Microservice for notifications",
  "category": "enterprise",
  "git": {
    "url": "https://github.com/org/new-service.git",
    "branch": "main"
  },
  "tags": ["notification"]
}
```

For agent projects, also include `agentId` and optionally `devGoal`.

**Response:** `201 Created` — `ProjectItem`.

### PATCH `/api/v1/projects/:id`

**Request:**

```json
{
  "description": "Updated description",
  "status": "stable",
  "tags": ["core-service", "payment", "v4"]
}
```

**Response:** `200 OK` — Updated `ProjectItem`.

### GET `/api/v1/projects/stats`

**Response:** `200 OK`

```json
{
  "totalProjects": 12,
  "byCategory": { "enterprise": 6, "agent": 6 },
  "byStatus": { "active": 5, "stable": 3, "developing": 3, "archived": 1 },
  "byLanguage": { "Rust": 4, "Go": 3, "TypeScript": 3, "Python": 2 },
  "totalCommits": 12450,
  "totalOpenPRs": 18
}
```

---

## 5. Enterprise Systems

Read-only view of enterprise systems that agents interact with. Systems are registered by administrators and represent the organization's technology landscape. Agents reference systems via `agentIds` to indicate which personas manage or monitor each system.

### Endpoints (3)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/systems` | List systems |
| GET | `/api/v1/systems/:id` | System detail |
| GET | `/api/v1/systems/counts` | Category counts |

### EnterpriseSystem Schema

```json
{
  "id": "finance-core",
  "name": "Finance Core System",
  "description": "Enterprise core financial management platform",
  "category": "finance",
  "status": "running",
  "icon": "circle-dollar-sign",
  "agentIds": ["financial-analyst", "fullstack-engineer"],
  "version": "3.2.1",
  "lastDeploy": 1707753600000,
  "uptime": "99.97%",
  "stack": ["React", "Rust", "PostgreSQL"],
  "metrics": [
    { "label": "Monthly processed", "value": "¥380M" },
    { "label": "Active accounts", "value": "12,847" }
  ],
  "modules": [
    {
      "name": "General Ledger",
      "description": "Multi-book, multi-currency account management",
      "status": "active"
    },
    {
      "name": "Smart Reconciliation",
      "description": "AI-powered automatic reconciliation engine",
      "status": "beta"
    }
  ]
}
```

`category`: `finance` | `risk` | `crm` | `compliance` | `data` | `internal`

`status`: `running` | `deploying` | `maintenance` | `offline`

`icon`: Lucide icon name (string). The frontend maps this to the corresponding icon component.

`modules[].status`: `active` | `beta` | `planned`

### GET `/api/v1/systems`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `category` | string | Filter by category |
| `status` | string | Filter by status |
| `q` | string | Search name/description |

**Response:** `200 OK` — `EnterpriseSystem[]` (no pagination; typically < 50 systems).

### GET `/api/v1/systems/:id`

**Response:** `200 OK` — Full `EnterpriseSystem` including `modules`.

### GET `/api/v1/systems/counts`

**Response:** `200 OK`

```json
{
  "finance": 2,
  "risk": 1,
  "crm": 1,
  "compliance": 1,
  "data": 1,
  "internal": 2,
  "total": 8,
  "byStatus": {
    "running": 6,
    "deploying": 1,
    "maintenance": 1,
    "offline": 0
  }
}
```

---

## 6. Agent Marketplace

A marketplace for discovering, hiring, and publishing AI agent personas. Includes a bounty system for posting and claiming development tasks.

### Endpoints (10)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/marketplace/agents` | Browse marketplace |
| GET | `/api/v1/marketplace/agents/:id` | Agent detail |
| POST | `/api/v1/marketplace/agents/:id/hire` | Hire agent |
| GET | `/api/v1/marketplace/my-agents` | My published agents |
| POST | `/api/v1/marketplace/my-agents` | Publish agent |
| PATCH | `/api/v1/marketplace/my-agents/:id` | Update published agent |
| GET | `/api/v1/marketplace/bounties` | List bounties |
| GET | `/api/v1/marketplace/bounties/:id` | Bounty detail |
| POST | `/api/v1/marketplace/bounties/:id/apply` | Apply for bounty |
| GET | `/api/v1/marketplace/stats` | Marketplace statistics |

### MarketAgent Schema

```json
{
  "id": "ma-1",
  "name": "QuantAlpha Pro",
  "provider": "DeepQuant Labs",
  "providerVerified": true,
  "description": "Professional quantitative factor mining agent",
  "category": "finance",
  "avatar": { "sex": "man", "faceColor": "#F9C9B6", "earSize": "small" },
  "tier": "pro",
  "price": 2800,
  "priceCurrency": "CNY",
  "priceUnit": "month",
  "rating": 4.8,
  "reviews": 127,
  "hires": 342,
  "capabilities": ["Multi-factor mining", "IC/IR analysis", "Portfolio optimization"],
  "tags": ["quantitative", "factor"],
  "featured": true,
  "teeSupported": true
}
```

`category`: `finance` | `risk` | `dev` | `data` | `ops` | `legal`

`tier`: `free` | `basic` | `pro` | `enterprise`

`avatar`: `react-nice-avatar` configuration object.

### BountyTask Schema

```json
{
  "id": "b-1",
  "title": "Build cross-border payment routing optimization engine",
  "description": "Design and implement an intelligent routing engine...",
  "reward": 25000,
  "rewardCurrency": "CNY",
  "difficulty": "hard",
  "category": "finance",
  "status": "open",
  "poster": "SafeClaw Finance",
  "posterVerified": true,
  "deadline": "2025-03-15",
  "applicants": 7,
  "requirements": ["Proficient in Rust/Go", "Payment industry experience"],
  "tags": ["payment", "routing", "TEE"]
}
```

`difficulty`: `easy` | `medium` | `hard` | `expert`

`status`: `open` | `in_progress` | `completed`

### GET `/api/v1/marketplace/agents`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `category` | string | Filter by category |
| `tier` | string | Filter by pricing tier |
| `featured` | bool | Featured agents only |
| `teeSupported` | bool | TEE-capable agents only |
| `q` | string | Search name/description/capabilities |
| `sortBy` | string | `rating\|hires\|price` (default: `rating`) |
| `sortOrder` | string | `asc\|desc` (default: `desc`) |
| `page` | number | Page number (default: 1) |
| `perPage` | number | Items per page (default: 20) |

**Response:** `200 OK` — Paginated `MarketAgent[]`

### GET `/api/v1/marketplace/agents/:id`

**Response:** `200 OK` — Full `MarketAgent` with additional fields:

```json
{
  "...all MarketAgent fields",
  "longDescription": "Detailed markdown description...",
  "changelog": "## v2.1.0\n- Added IC/IR analysis...",
  "screenshots": ["https://..."],
  "systemPrompt": "You are a quantitative analyst..."
}
```

### POST `/api/v1/marketplace/agents/:id/hire`

Hire a marketplace agent. This creates a local persona based on the marketplace agent's configuration.

**Request:**

```json
{
  "customName": "My Quant Agent"
}
```

All fields optional. `customName` overrides the default agent name.

**Response:** `201 Created`

```json
{
  "personaId": "quant-alpha-pro-1",
  "marketAgentId": "ma-1",
  "name": "My Quant Agent",
  "status": "hired"
}
```

### GET `/api/v1/marketplace/my-agents`

List agents published by the current user.

**Response:** `200 OK`

```json
[
  {
    "id": "pub-1",
    "personaId": "financial-analyst",
    "name": "Financial Analyst Pro",
    "description": "Enterprise financial analysis agent",
    "tier": "pro",
    "price": 1500,
    "priceUnit": "month",
    "hires": 89,
    "revenue": 133500,
    "rating": 4.6,
    "reviews": 34,
    "status": "published"
  }
]
```

`status`: `published` | `draft` | `paused`

### POST `/api/v1/marketplace/my-agents`

Publish a local persona to the marketplace.

**Request:**

```json
{
  "personaId": "custom-analyst",
  "description": "Specialized compliance analysis agent",
  "category": "compliance",
  "tier": "basic",
  "price": 500,
  "priceCurrency": "CNY",
  "priceUnit": "month",
  "capabilities": ["AML screening", "KYC verification"],
  "tags": ["compliance", "AML"],
  "teeSupported": false
}
```

**Response:** `201 Created` — `MyPublishedAgent`.

### PATCH `/api/v1/marketplace/my-agents/:id`

**Request:**

```json
{
  "price": 600,
  "status": "paused",
  "description": "Updated description"
}
```

**Response:** `200 OK` — Updated `MyPublishedAgent`.

### GET `/api/v1/marketplace/bounties`

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `status` | string | `open\|in_progress\|completed` |
| `difficulty` | string | `easy\|medium\|hard\|expert` |
| `category` | string | Filter by category |
| `q` | string | Search title/description |
| `page` | number | Page number (default: 1) |
| `perPage` | number | Items per page (default: 20) |

**Response:** `200 OK` — Paginated `BountyTask[]`

### GET `/api/v1/marketplace/bounties/:id`

**Response:** `200 OK` — Full `BountyTask`.

### POST `/api/v1/marketplace/bounties/:id/apply`

Apply for a bounty task.

**Request:**

```json
{
  "personaId": "fullstack-engineer",
  "proposal": "I can build this using Rust with..."
}
```

**Response:** `200 OK`

```json
{
  "bountyId": "b-1",
  "applicationId": "app-1",
  "status": "applied",
  "appliedAt": 1707753600000
}
```

### GET `/api/v1/marketplace/stats`

**Response:** `200 OK`

```json
{
  "totalAgents": 156,
  "totalHires": 4280,
  "totalBounties": 42,
  "openBounties": 18,
  "totalBountyValue": 850000,
  "byCategory": {
    "finance": 45,
    "risk": 28,
    "dev": 38,
    "data": 22,
    "ops": 15,
    "legal": 8
  },
  "featuredCount": 12
}
```

---

## 7. Settings

Application settings management. API keys are stored server-side and returned in masked form. The settings model covers LLM provider configuration, gateway behavior, and UI preferences.

### Endpoints (4)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/settings` | Get settings |
| PATCH | `/api/v1/settings` | Update settings |
| POST | `/api/v1/settings/reset` | Reset to defaults |
| GET | `/api/v1/settings/info` | Server info |

### Settings Schema

```json
{
  "provider": "anthropic",
  "model": "claude-sonnet-4-20250514",
  "baseUrl": "",
  "apiKey": "sk-ant-...****7f3a",
  "gateway": {
    "listenAddr": "127.0.0.1:18790",
    "teeEnabled": false,
    "corsOrigins": ["http://localhost:1420"]
  },
  "privacy": {
    "classificationEnabled": true,
    "sensitivePatterns": ["SSN", "credit_card"],
    "redactionEnabled": false
  },
  "storage": {
    "backend": "file",
    "sessionsDir": "~/.safeclaw/sessions"
  }
}
```

### GET `/api/v1/settings`

Returns current settings. API keys are masked (first 8 + last 4 characters visible).

**Response:** `200 OK` — `Settings` object.

### PATCH `/api/v1/settings`

Partial update. Only provided fields are changed.

**Request:**

```json
{
  "provider": "openai",
  "model": "gpt-4o",
  "apiKey": "sk-proj-..."
}
```

When `apiKey` is provided, it is stored in full. The response returns the masked version.

**Response:** `200 OK` — Updated `Settings` (with masked API key).

### POST `/api/v1/settings/reset`

Reset all settings to defaults. This does not delete sessions or knowledge data.

**Response:** `200 OK` — Default `Settings`.

### GET `/api/v1/settings/info`

Server runtime information.

**Response:** `200 OK`

```json
{
  "version": "0.3.1",
  "buildDate": "2025-01-15",
  "rustVersion": "1.83.0",
  "os": "macos-aarch64",
  "uptime": 86400,
  "sessionsDir": "/Users/user/.safeclaw/sessions",
  "configPath": "/Users/user/.safeclaw/config.toml",
  "a3sCodeVersion": "0.3.1",
  "features": {
    "tee": false,
    "privacy": true,
    "gateway": true
  }
}
```

---

## 8. Common Endpoints

### Health & Status (existing, no v1 prefix)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/status` | Gateway status |

#### GET `/health`

**Response:** `200 OK`

```json
{
  "status": "ok",
  "version": "0.3.1"
}
```

#### GET `/status`

**Response:** `200 OK`

```json
{
  "state": "Running",
  "teeEnabled": false,
  "sessionCount": 3,
  "channels": ["webchat"],
  "a3sGatewayMode": false
}
```

### Personas (4)

Personas are agent identities with avatars, system prompts, and default configurations. They are the universal foreign key across events, systems, projects, and marketplace. SafeClaw ships with 13 builtin personas; users can create custom ones.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/personas` | List all personas |
| GET | `/api/v1/personas/:id` | Persona detail |
| POST | `/api/v1/personas` | Create custom persona |
| PATCH | `/api/v1/personas/:id` | Update persona |

#### AgentPersona Schema

```json
{
  "id": "financial-analyst",
  "name": "Financial Analyst",
  "description": "Senior financial analysis and reporting specialist",
  "avatar": {
    "sex": "woman",
    "faceColor": "#F9C9B6",
    "earSize": "small",
    "eyeStyle": "circle",
    "noseStyle": "round",
    "mouthStyle": "smile",
    "shirtStyle": "polo",
    "glassesStyle": "none",
    "hairColor": "#000",
    "hairStyle": "womanLong",
    "hatStyle": "none",
    "hatColor": "#000",
    "eyeBrowStyle": "up",
    "shirtColor": "#6BD9E9",
    "bgColor": "#E0DDFF"
  },
  "systemPrompt": "You are a senior financial analyst...",
  "defaultModel": "claude-sonnet-4-20250514",
  "defaultPermissionMode": "default",
  "builtin": true,
  "undeletable": true
}
```

`avatar`: `react-nice-avatar` full configuration object. The frontend renders this directly.

`builtin`: `true` for the 13 shipped personas, `false` for user-created ones.

`undeletable`: `true` for personas that cannot be removed (core system personas).

#### GET `/api/v1/personas`

**Response:** `200 OK` — `AgentPersona[]`

Returns all personas (builtin + custom). No pagination (typically < 50).

#### GET `/api/v1/personas/:id`

**Response:** `200 OK` — `AgentPersona`.

#### POST `/api/v1/personas`

**Request:**

```json
{
  "name": "Tax Specialist",
  "description": "Corporate tax planning and compliance",
  "avatar": { "sex": "man", "faceColor": "#F9C9B6" },
  "systemPrompt": "You are a tax specialist...",
  "defaultModel": "claude-sonnet-4-20250514",
  "defaultPermissionMode": "default"
}
```

`id` is auto-generated from `name` (kebab-case).

**Response:** `201 Created` — `AgentPersona`.

#### PATCH `/api/v1/personas/:id`

Update a custom persona. Builtin personas cannot be modified (returns `403`).

**Request:**

```json
{
  "name": "Senior Tax Specialist",
  "systemPrompt": "Updated prompt..."
}
```

**Response:** `200 OK` — Updated `AgentPersona`.

### User Profile (1)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/user/profile` | Current user info |

#### GET `/api/v1/user/profile`

**Response:** `200 OK`

```json
{
  "id": 1,
  "nickname": "Roy Lin",
  "email": "admin@elljs.com",
  "avatar": "https://github.com/user.png"
}
```

Currently returns a hardcoded user. Will be backed by auth system in the future.

---

## 9. WebSocket Protocol

The primary real-time channel is `/ws/agent/browser/:id`, connecting the browser to an agent session. The protocol uses JSON messages with a `type` discriminator.

### Connection Lifecycle

```
Browser                          Server
  │                                │
  ├── WS CONNECT ────────────────→ │
  │                                ├── session_init (AgentSessionState)
  │                                ├── message_history (if reconnecting)
  │  ←──────────────────────────── │
  │                                │
  ├── user_message ──────────────→ │
  │                                ├── SessionManager.generate_streaming()
  │                                ├── stream_event (TextDelta, ToolStart, ...)
  │                                ├── assistant (complete message)
  │                                ├── tool_progress / tool_use_summary
  │                                ├── permission_request (if HITL needed)
  │  ←──────────────────────────── │
  │                                │
  ├── permission_response ───────→ │  (allow/deny tool execution)
  ├── interrupt ─────────────────→ │  (cancel current generation)
  ├── set_model ─────────────────→ │  (switch model mid-session)
  ├── set_permission_mode ───────→ │  (change permission policy)
  │                                │
```

### Server → Browser Messages

#### `session_init`

Sent immediately after WebSocket connection. Contains the full session state.

```json
{
  "type": "session_init",
  "sessionState": {
    "session_id": "sess-a1b2c3",
    "model": "claude-sonnet-4-20250514",
    "cwd": "/home/user/project",
    "tools": ["bash", "read", "write", "edit", "grep", "glob", "ls"],
    "permission_mode": "default",
    "mcp_servers": [{ "name": "github", "status": "connected" }],
    "agents": [],
    "slash_commands": [],
    "skills": ["code-review"],
    "total_cost_usd": 0.0,
    "num_turns": 0,
    "context_used_percent": 0.0,
    "is_compacting": false,
    "total_lines_added": 0,
    "total_lines_removed": 0
  }
}
```

#### `session_update`

Partial update to session state (e.g., after model switch, cost change).

```json
{
  "type": "session_update",
  "sessionState": {
    "total_cost_usd": 0.042,
    "num_turns": 3,
    "context_used_percent": 12.5
  }
}
```

#### `assistant`

A complete assistant message (sent after streaming finishes or for non-streamed responses).

```json
{
  "type": "assistant",
  "message": {
    "id": "msg-1",
    "role": "assistant",
    "content": "I'll help you refactor that module.",
    "contentBlocks": [
      { "type": "text", "text": "I'll help you refactor that module." }
    ],
    "timestamp": "2025-01-15T10:30:00Z",
    "model": "claude-sonnet-4-20250514",
    "stopReason": "end_turn"
  }
}
```

#### `stream_event`

Real-time streaming events from the agent loop. Maps from a3s-code `AgentEvent` variants.

```json
{ "type": "stream_event", "event": "turn_start" }
{ "type": "stream_event", "event": "text_delta", "delta": "Here's the " }
{ "type": "stream_event", "event": "text_delta", "delta": "refactored code:" }
{ "type": "stream_event", "event": "tool_start", "toolName": "write", "toolUseId": "tu-1" }
{ "type": "stream_event", "event": "tool_end", "toolUseId": "tu-1" }
{ "type": "stream_event", "event": "turn_end" }
```

#### `result`

Generation completed.

```json
{
  "type": "result",
  "result": "Generation completed successfully",
  "subtype": "success"
}
```

`subtype`: `success` | `error` | `interrupted` | `max_turns`

#### `permission_request`

HITL confirmation required for a tool call. The UI must display this and send back a `permission_response`.

```json
{
  "type": "permission_request",
  "permission": {
    "request_id": "perm-1",
    "tool_name": "bash",
    "input": { "command": "rm -rf /tmp/old-build" },
    "description": "Execute shell command",
    "tool_use_id": "tu-2",
    "timestamp": "2025-01-15T10:30:05Z"
  }
}
```

#### `permission_cancelled`

A previously requested permission is no longer needed (e.g., generation was interrupted).

```json
{
  "type": "permission_cancelled",
  "requestId": "perm-1"
}
```

#### `tool_progress`

Progress update during long-running tool execution.

```json
{
  "type": "tool_progress",
  "toolUseId": "tu-1",
  "toolName": "bash",
  "progress": "Running tests... 42/100 passed"
}
```

#### `tool_use_summary`

Summary after a tool call completes.

```json
{
  "type": "tool_use_summary",
  "toolUseId": "tu-1",
  "toolName": "write",
  "summary": "Wrote 45 lines to src/handler.rs",
  "isError": false
}
```

#### `status_change`

Session status changed (e.g., compacting context).

```json
{
  "type": "status_change",
  "status": "compacting"
}
```

#### `error`

An error occurred during generation.

```json
{
  "type": "error",
  "error": "Rate limit exceeded, retrying in 30s",
  "code": "RATE_LIMIT"
}
```

#### `user_message`

Echo of the user's message (for multi-device sync / history replay).

```json
{
  "type": "user_message",
  "message": {
    "id": "msg-0",
    "role": "user",
    "content": "Refactor the auth module",
    "timestamp": "2025-01-15T10:29:55Z"
  }
}
```

#### `message_history`

Full message history replay (sent on reconnection).

```json
{
  "type": "message_history",
  "messages": [
    { "id": "msg-0", "role": "user", "content": "...", "timestamp": "..." },
    { "id": "msg-1", "role": "assistant", "content": "...", "timestamp": "..." }
  ]
}
```

#### `session_name_update`

Auto-generated session name (from `SessionManager::generate_title()`).

```json
{
  "type": "session_name_update",
  "sessionId": "sess-a1b2c3",
  "name": "Auth module refactor"
}
```

### Browser → Server Messages

#### `user_message`

Send a message to the agent. Triggers `SessionManager::generate_streaming()`.

```json
{
  "type": "user_message",
  "content": "Refactor the auth module to use JWT",
  "images": [
    { "media_type": "image/png", "data": "base64..." }
  ]
}
```

`images` is optional. Supports vision-capable models.

#### `permission_response`

Respond to a `permission_request`.

```json
{
  "type": "permission_response",
  "request_id": "perm-1",
  "allowed": true
}
```

#### `interrupt`

Cancel the current generation. Calls `SessionManager::cancel_operation()`.

```json
{
  "type": "interrupt"
}
```

#### `set_model`

Switch the LLM model for this session. Calls `SessionManager::configure()`.

```json
{
  "type": "set_model",
  "model": "claude-sonnet-4-20250514"
}
```

#### `set_permission_mode`

Change the permission policy for this session.

```json
{
  "type": "set_permission_mode",
  "mode": "auto-accept"
}
```

`mode`: `default` | `auto-accept` | `deny-all`

---

## Implementation Priority

| Phase | Scope | Endpoints | Description |
|-------|-------|-----------|-------------|
| P0 | Agent Chat + Settings + Personas | 17 | Already implemented. Migrate to `/api/v1` prefix. |
| P1 | Knowledge Base + Events | 13 | Core business functionality. Knowledge storage and event routing. |
| P2 | Projects + Systems | 10 | Asset management and enterprise system registry. |
| P3 | Marketplace | 10 | Agent marketplace ecosystem (hire, publish, bounties). |

### P0 → P1 Migration Notes

The existing agent endpoints (`/api/agent/sessions/...`) should be aliased to `/api/v1/agent/sessions/...`. The old paths can remain as deprecated aliases during the transition period.

### Backend Dependencies

| Domain | Storage | a3s-code Integration |
|--------|---------|---------------------|
| Agent Chat | `SessionManager` (in-memory + file persistence) | Direct — all CRUD and generation |
| Events | New `EventStore` (file-based JSON or SQLite) | Indirect — agents can create events via tool calls |
| Knowledge | New `KnowledgeStore` (filesystem + metadata index) | Indirect — agents can read knowledge items as context |
| Projects | New `ProjectStore` (git repo metadata + file access) | Indirect — agent projects track `agentId` |
| Systems | New `SystemRegistry` (config file or DB) | Indirect — systems reference `agentIds` |
| Marketplace | New `MarketplaceStore` (DB or remote API) | Indirect — hiring creates local personas |
| Settings | `SafeClawConfig` (TOML file) | Direct — model/provider config feeds into `CodeConfig` |
| Personas | `PersonaStore` (builtin JSON + custom file) | Direct — persona's `systemPrompt` and `defaultModel` used in session creation |
