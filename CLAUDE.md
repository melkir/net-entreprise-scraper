# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust web scraper service deployed on Cloudflare Workers that monitors the net-entreprises.fr website for DSN validation tool updates. It exposes a simple HTTP API that returns information about the latest version, including build number, release date, and download URL.

## Architecture

The application consists of two main modules:
- `src/lib.rs`: Cloudflare Worker entry point with fetch event handler and routing
- `src/client.rs`: Web scraping logic using scraper crate to parse HTML and extract version information

The scraper targets specific selectors (`strong > a` for links, `td > p > strong` for version text) and parses French month names to ISO date format. Uses the workers-rs crate for HTTP requests and error handling.

## Prerequisites

- Recent version of Rust with `wasm32-unknown-unknown` target
- Node.js and npm (for Wrangler CLI)
- `worker-build` tool (installed automatically during build)

```bash
# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Wrangler CLI globally
npm install -g wrangler
```

## Development Commands

### Local Development
```bash
# Install worker-build and run locally
wrangler dev

# Build for production
wrangler build
```

### Code Quality
```bash
# Format code
cargo fmt

# Check code (for WebAssembly target)
cargo check --target wasm32-unknown-unknown

# Run clippy linter
cargo clippy --target wasm32-unknown-unknown
```

### Build Process
The build is handled by `worker-build` which:
- Compiles Rust to WebAssembly
- Generates JavaScript shim for Worker runtime
- Optimizes binary size with `wasm-opt`

## Deployment

### Cloudflare Workers
```bash
# Deploy to Cloudflare Workers
wrangler deploy

# View logs
wrangler tail
```

Configuration is managed via `wrangler.toml`. The worker compiles to WebAssembly and runs on Cloudflare's edge network.

## API

- `GET /`: Returns JSON with current DSN tool version info
  ```json
  {
    "version": "build_number",
    "date": "YYYY-MM-DD",
    "url": "download_link"
  }
  ```

The worker automatically handles HTTPS and runs globally on Cloudflare's edge network.