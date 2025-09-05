# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust web scraper service that monitors the net-entreprises.fr website for DSN validation tool updates. It exposes a simple HTTP API that returns information about the latest version, including build number, release date, and download URL.

## Architecture

The application consists of two main modules:
- `src/main.rs`: Axum web server with a single JSON endpoint at `/`
- `src/client.rs`: Web scraping logic using scraper crate to parse HTML and extract version information

The scraper targets specific selectors (`strong > a` for links, `td > p > strong` for version text) and parses French month names to ISO date format.

## Development Commands

### Build and Run
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run locally
cargo run

# Run with custom port
PORT=3000 cargo run
```

### Code Quality
```bash
# Format code
cargo fmt

# Check code
cargo check

# Run clippy linter
cargo clippy
```

### Docker
```bash
# Build Docker image
docker build -t net-entreprise-scraper .

# Run with Docker
docker run -p 8000:8000 net-entreprise-scraper
```

## Deployment

The service is configured for Render.com deployment via `render.yaml`. It uses a multi-stage Docker build with the final image running on Arch Linux base.

## API

- `GET /`: Returns JSON with current DSN tool version info
  ```json
  {
    "version": "build_number",
    "date": "YYYY-MM-DD",
    "url": "download_link"
  }
  ```

The server listens on port 8000 by default, configurable via `PORT` environment variable.