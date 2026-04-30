# Architecture Decisions

Decisions made during implementation, with rationale. Referenced from CLAUDE.md.

## ADR-001: Rust CLI + Rust Supervisor + Tauri Shell

**Decision:** Single language (Rust) for all backend binaries, Tauri v2 + React for the desktop shell.

**Rationale:** Shared manifest parsing, runtime code, and filesystem utilities across CLI and supervisor. Small native binaries. Natural pairing with Tauri. Cross-platform future path without rewriting.

**Trade-off:** Slower iteration than Go/Node for CLI, but long-term binary/library cohesion wins.

## ADR-002: Supervisor as child process of shell (not Windows service)

**Decision:** The Tauri shell spawns `craterun-supervisor.exe` as a child process.

**Rationale:** Windows services require admin rights to install. Per-user install with no elevation is a hard product requirement. A child process requires nothing special. If the shell crashes, the supervisor keeps containers alive — the user relaunches the shell and it reconnects via `supervisor.sock.json`.

**Future:** May revisit for background-service mode if users want apps to survive logoff.

## ADR-003: Loopback HTTP with random Bearer token for IPC

**Decision:** Supervisor exposes an axum HTTP API on `127.0.0.1:<random>` with a one-time token.

**Rationale:** Named pipes are more secure on Windows but harder to debug and not cross-platform. Loopback HTTP with NTFS-protected token file achieves equivalent security. Trivial to implement with axum. Easy to curl during development.

**Token file:** `%LOCALAPPDATA%\CrateRun\Apps\<appId>\config\supervisor.sock.json`

## ADR-004: Per-app WSL distro isolation

**Decision:** Each installed app gets its own WSL2 distro named `craterun-<appId>`.

**Rationale:** Isolates apps from each other. Enables clean uninstall via `wsl --unregister`. Avoids version conflicts between apps needing different containerd versions. Trade-off is disk space (~200MB base per app) but this is the correct isolation model for consumer apps.

## ADR-005: Embedded reverse proxy (not separate binary)

**Decision:** The supervisor hosts the reverse proxy in-process using hyper/reqwest.

**Rationale:** One fewer process to manage, one fewer failure point. The supervisor controls the proxy lifecycle naturally. Routes are derived from `manifest.json` at startup. Single random port on 127.0.0.1 — the user never sees it.

## ADR-006: Consumer subset validator as a hard gate

**Decision:** `craterun certify` and `craterun package --consumer` reject Compose files with unsafe features.

**Rationale:** If developers can package arbitrary Compose files, the consumer experience and security model break. The validator blocks privileged containers, host networking, Docker socket mounts, fixed host ports, and other unsafe patterns. Dev Pack mode exists as an escape hatch for developers who need those features.

## ADR-007: Error translation as a single choke point

**Decision:** All `CrateRunError` → `UserFacingError` conversion happens in one `impl From` block in `craterun-core/src/error.rs`.

**Rationale:** Consumer-facing error messages are a product surface, not an afterthought. A single conversion point ensures every infrastructure error has a human translation. No other code path is allowed to construct `user_title` or `user_message` directly.

## ADR-008: NSIS for Windows installer generation (planned M5)

**Decision:** Use NSIS (Nullsoft Scriptable Install System) for generating `.exe` installers.

**Rationale:** NSIS handles UAC elevation, per-user install, Add/Remove Programs registration, and uninstallation out of the box. Writing a custom self-extractor would reinvent all of this. The CLI generates NSIS scripts programmatically from the manifest.

## ADR-009: WSL2 + containerd + nerdctl for managed runtime (M4)

**Decision:** The consumer runtime is a private WSL2 distro with containerd and nerdctl pre-installed.

**Rationale:** Docker Desktop introduces licensing, account, install, startup, update, and UX friction. End users should not install Docker to run a normal app. A managed WSL2 distro with containerd is invisible to the user and fully controllable by the supervisor. The `wsl --import` command works per-user without admin rights.

**Risk:** This is the highest-risk technical decision. Potential blockers: virtualization disabled, WSL not installed, Windows feature requires reboot, enterprise policy, Defender interference.
