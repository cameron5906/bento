# Bento

A developer tool that turns Docker Compose apps into consumer-grade Windows desktop installers.

## Project Structure

Rust workspace with Tauri v2 desktop shell:

```
crates/
  bento-core/       # Zero-dep shared types: AppId, state machine, errors, paths
  bento-bundle/     # Manifest parser, Compose parser, validator, bundle I/O
  bento-runtime/    # RuntimeAdapter trait + adapter implementations
  bento-supervisor/ # Local supervisor binary (axum API, reverse proxy, state machine)
  bento-cli/        # Developer CLI binary (clap commands, build pipeline)
apps/
  shell-tauri/         # Tauri v2 + React desktop shell
examples/
  hello-web-api/       # Reference test app (React + Node API + Postgres)
schemas/               # JSON Schemas for bento.yml and manifest.json
```

## Architecture

Three concentric rings:

1. **Data Contracts** (`bento-core`, `bento-bundle`): Pure types, no async. Manifest schema, state machine, error translation.
2. **Runtime Execution** (`bento-runtime`, `bento-supervisor`): Async machinery. Runtime adapter trait, state machine executor, reverse proxy, health checks.
3. **Surfaces** (`bento-cli`, `shell-tauri`): Developer CLI and consumer desktop shell. Both thin over Ring 2.

## Key Conventions

- **Language**: Rust for all backend crates, TypeScript/React for shell UI
- **Async runtime**: tokio (everywhere except bento-core which is sync-only)
- **Error handling**: `thiserror` for internal errors, `UserFacingError` for API boundary
- **Serialization**: serde throughout, serde_yaml for manifests, serde_json for API
- **CLI framework**: clap with derive macros
- **HTTP server**: axum (supervisor API)
- **HTTP proxy**: hyper (embedded reverse proxy)

## Build Commands

```bash
cargo build                    # Build all crates
cargo test                     # Run all tests
cargo run -p bento-cli      # Run the CLI
```

## Tracking

- **[docs/decisions.md](docs/decisions.md)** — Architecture Decision Records (ADRs). All non-obvious decisions with rationale.
- **[docs/milestones.md](docs/milestones.md)** — Milestone tracker with status, commits, and definition of done.
- **[DOC.pdf](DOC.pdf)** — Original 39-page implementation brief (the spec).

## Design Rules

- Manifest is app-oriented, not infrastructure-oriented (`routes` not `hostPort`)
- Consumer Pack must be strict: no privileged containers, host networking, docker socket mounts
- All infrastructure errors must be translated to consumer-friendly messages via `UserFacingError`
- Health checks are mandatory for consumer packaging
- Bind only to 127.0.0.1, random high ports
- `bento-core` has NO async dependencies — it must remain usable from sync contexts
- The supervisor is a child process of the shell (not a Windows service)
- Shell-to-supervisor IPC: loopback HTTP with random bearer token
- Per-app WSL distro isolation (`bento-<appId>`)
