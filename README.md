# SafeClaw

<p align="center">
  <strong>Secure Personal AI Assistant with TEE Support</strong>
</p>

<p align="center">
  <em>Privacy-focused AI assistant that runs sensitive computations in hardware-isolated environments</em>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#roadmap">Roadmap</a>
</p>

---

## Overview

**SafeClaw** is a secure version of [OpenClaw](https://github.com/openclaw/openclaw) built on the A3S ecosystem. It combines multi-channel messaging capabilities with hardware-isolated execution environments (TEE) for processing sensitive data.

### What SafeClaw Does

- **Multi-Channel Messaging**: Connect to Telegram, Slack, Discord, WebChat, and more
- **Privacy Classification**: Automatically detect sensitive data (credit cards, SSN, emails, API keys)
- **TEE Processing**: Route sensitive computations to hardware-isolated A3S Box environments
- **Secure Communication**: End-to-end encryption between gateway and TEE

### What SafeClaw Does NOT Do

- Replace your existing AI assistant (it enhances privacy protection)
- Store sensitive data in plaintext (everything is encrypted)
- Process highly sensitive data outside TEE (configurable policy)

## Features

- **Hardware Isolation**: Sensitive data processing in A3S Box MicroVM
- **Automatic Classification**: Regex-based detection of PII and secrets
- **Policy Engine**: Configurable rules for data routing decisions
- **Multi-Channel Support**: Telegram, WebChat (Slack, Discord planned)
- **Secure Channels**: X25519 key exchange + AES-256-GCM encryption
- **Session Management**: Per-user sessions with sensitivity tracking

## Quick Start

### Prerequisites

- **Rust 1.75+**
- **A3S Box** (for TEE support)

### Installation

```bash
# Clone the repository
git clone https://github.com/A3S-Lab/SafeClaw.git
cd SafeClaw

# Build
cargo build --release

# Run
./target/release/safeclaw --help
```

### Basic Usage

```bash
# Start the gateway
safeclaw gateway --port 18790

# Run diagnostics
safeclaw doctor

# Show configuration
safeclaw config --default
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SafeClaw Gateway                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Channel Manager                           │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │   │
│  │  │ Telegram │ │  Slack   │ │ Discord  │ │   WebChat    │   │   │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └──────┬───────┘   │   │
│  └───────┼────────────┼────────────┼──────────────┼───────────┘   │
│          └────────────┴────────────┴──────────────┘               │
│                              │                                     │
│  ┌───────────────────────────▼───────────────────────────────┐   │
│  │                   Session Router                           │   │
│  │  - Route messages to appropriate TEE sessions              │   │
│  │  - Handle multi-agent routing                              │   │
│  │  - Manage session lifecycle                                │   │
│  └───────────────────────────┬───────────────────────────────┘   │
│                              │                                     │
│  ┌───────────────────────────▼───────────────────────────────┐   │
│  │                   Privacy Classifier                       │   │
│  │  - Classify data sensitivity                               │   │
│  │  - Route sensitive data to TEE                             │   │
│  │  - Handle encryption/decryption                            │   │
│  └───────────────────────────┬───────────────────────────────┘   │
└──────────────────────────────┼────────────────────────────────────┘
                               │ vsock / encrypted channel
┌──────────────────────────────▼────────────────────────────────────┐
│                    TEE Environment (A3S Box)                       │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    Secure Agent Runtime                      │  │
│  │  ┌─────────────────┐  ┌─────────────────────────────────┐   │  │
│  │  │  A3S Code Agent │  │     Secure Data Store           │   │  │
│  │  │  - LLM Client   │  │  - Encrypted credentials        │   │  │
│  │  │  - Tool Exec    │  │  - Private conversation history │   │  │
│  │  │  - HITL         │  │  - Sensitive user data          │   │  │
│  │  └─────────────────┘  └─────────────────────────────────┘   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                         MicroVM (Hardware Isolated)                │
└────────────────────────────────────────────────────────────────────┘
```

## Configuration

SafeClaw uses TOML configuration files. Default location: `~/.config/safeclaw/config.toml`

### Example Configuration

```toml
[gateway]
host = "127.0.0.1"
port = 18790
tls_enabled = false

[tee]
enabled = true
backend = "a3s_box"
box_image = "ghcr.io/a3s-lab/safeclaw-tee:latest"
memory_mb = 2048
cpu_cores = 2

[privacy]
auto_classify = true
default_level = "normal"

[[privacy.rules]]
name = "credit_card"
pattern = '\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b'
level = "highly_sensitive"
description = "Credit card numbers"

[[privacy.rules]]
name = "api_key"
pattern = '\b(sk-|api[_-]?key|token)[A-Za-z0-9_-]{20,}\b'
level = "highly_sensitive"
description = "API keys and tokens"

[models]
default_provider = "anthropic"

[models.providers.anthropic]
api_key_ref = "anthropic_api_key"
default_model = "claude-sonnet-4-20250514"
```

### Privacy Classification Rules

Built-in rules detect:
- Credit card numbers
- Social Security Numbers (SSN)
- Email addresses
- Phone numbers
- API keys and tokens

### Sensitivity Levels

| Level | Description | Processing |
|-------|-------------|------------|
| `public` | Non-sensitive data | Local processing |
| `normal` | Default level | Local processing |
| `sensitive` | PII, contact info | TEE processing |
| `highly_sensitive` | Financial, credentials | TEE processing + extra protection |

## CLI Commands

```bash
# Start the gateway server
safeclaw gateway [--host HOST] [--port PORT] [--no-tee]

# Run onboarding wizard
safeclaw onboard [--install-daemon]

# Send a message
safeclaw message --channel CHANNEL --to CHAT_ID --message TEXT

# Run diagnostics
safeclaw doctor

# Show configuration
safeclaw config [--default]
```

## Project Structure

```
safeclaw/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library entry point
│   ├── main.rs             # CLI entry point
│   ├── config.rs           # Configuration management
│   ├── error.rs            # Error types
│   ├── channels/           # Multi-channel adapters
│   │   ├── adapter.rs      # Channel adapter trait
│   │   ├── message.rs      # Message types
│   │   ├── telegram.rs     # Telegram adapter
│   │   └── webchat.rs      # WebChat adapter
│   ├── crypto/             # Cryptographic utilities
│   │   ├── keys.rs         # Key management
│   │   └── secure_channel.rs # Encrypted channels
│   ├── gateway/            # Gateway server
│   │   ├── server.rs       # Main gateway
│   │   ├── handler.rs      # HTTP API
│   │   └── websocket.rs    # WebSocket handler
│   ├── privacy/            # Privacy classification
│   │   ├── classifier.rs   # Sensitive data detection
│   │   └── policy.rs       # Policy engine
│   ├── session/            # Session management
│   │   ├── manager.rs      # Session lifecycle
│   │   └── router.rs       # Privacy-based routing
│   └── tee/                # TEE integration
│       ├── client.rs       # TEE client
│       ├── manager.rs      # TEE session management
│       └── protocol.rs     # Communication protocol
```

## Roadmap

### Phase 1: Foundation ✅

- [x] Project structure and configuration
- [x] Privacy classifier with regex rules
- [x] Policy engine for routing decisions
- [x] Session management
- [x] Cryptographic utilities (X25519, AES-GCM)
- [x] TEE client and protocol

### Phase 2: Channels 🚧

- [x] Channel adapter trait
- [x] Telegram adapter (skeleton)
- [x] WebChat adapter
- [ ] Slack adapter
- [ ] Discord adapter

### Phase 3: Gateway 🚧

- [x] Gateway server structure
- [x] HTTP API endpoints
- [x] WebSocket handler
- [ ] Full Telegram Bot API integration
- [ ] Authentication and authorization

### Phase 4: TEE Integration 📋

- [ ] A3S Box integration
- [ ] Secure channel establishment
- [ ] Remote attestation
- [ ] Secure credential storage

### Phase 5: Production 📋

- [ ] Comprehensive testing
- [ ] Performance optimization
- [ ] Documentation
- [ ] Docker images
- [ ] Kubernetes deployment

## A3S Ecosystem

SafeClaw is part of the A3S ecosystem:

| Project | Description |
|---------|-------------|
| [A3S Box](https://github.com/A3S-Lab/Box) | MicroVM sandbox runtime with hardware isolation |
| [A3S Code](https://github.com/A3S-Lab/Code) | AI coding agent with tool execution |
| [A3S Lane](https://github.com/A3S-Lab/Lane) | Priority-based command queue |
| [A3S Context](https://github.com/A3S-Lab/Context) | Hierarchical context management |
| **SafeClaw** | Secure personal AI assistant with TEE support |

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo fmt
cargo clippy
```

## License

MIT

---

<p align="center">
  Built by <a href="https://github.com/A3S-Lab">A3S Lab</a>
</p>
