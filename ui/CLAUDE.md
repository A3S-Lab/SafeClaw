# CLAUDE.md

This file provides guidance to Claude Code when working with the SafeClaw UI.

## Project Overview

SafeClaw UI is the desktop frontend for SafeClaw (Secure Personal AI Assistant with TEE Support). Built with React + TypeScript + Tailwind CSS, packaged as a native desktop app via Tauri v2.

## Development Commands

```bash
# Install dependencies (pnpm only)
pnpm install

# Start frontend dev server only (port 8888)
pnpm dev

# Start Tauri dev mode (frontend + native window)
pnpm tauri:dev

# Build production Tauri app
pnpm tauri:build

# Format code with Biome
pnpm format
```

## Architecture

- **Build tool:** Rsbuild (Rspack-based)
- **Desktop runtime:** Tauri v2
- **State management:** Valtio
- **UI components:** shadcn/ui + Radix UI
- **Routing:** React Router v7 (hash router for Tauri compatibility)
- **Gateway communication:** HTTP fetch to SafeClaw gateway at `PUBLIC_GATEWAY_URL`

## Key Directories

- `src/` - React frontend source
- `src-tauri/` - Tauri Rust backend
- `env/` - Environment variables
