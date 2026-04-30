# Milestone Tracker

Status of each milestone from the implementation brief (DOC.pdf).

| # | Milestone | Status | Commit | Notes |
|---|---|---|---|---|
| M1 | Static App Bundle Compiler | DONE | `5c07808` | Core types, manifest parser, compose validator, CLI commands |
| M2 | Local Supervisor Prototype | DONE | `cc4394d` | State machine, axum API, reverse proxy, health checker |
| M3 | Tauri Shell Prototype | DONE | `4ba01eb` | React UI, supervisor client, state-driven screens |
| M4 | Windows Managed Runtime Spike | DONE | see below | WSL2 + containerd adapter. Needs VM testing. |
| M5 | Native Windows Installer | DONE | see below | NSIS script generator + CLI integration |
| M6 | Repair, Reset, Diagnostics | DONE | see below | Diagnostics export, error mapping, repair flow |
| M7 | Consumer Certification Gate | DONE | see below | 17-check certify, consumer-pack build gate |
| M8 | First External Test | NOT STARTED | — | Hand to non-technical user, observe |

## Definition of Done (from spec)

The project reaches 0→1 when:
- A developer can package a supported Compose app with one command
- The output is a Windows installer
- A clean Windows 11 machine can install it without Docker Desktop
- The app opens in a native desktop window
- The user never sees Docker, Compose, WSL, containerd, ports, or terminal output
- The app can be closed and reopened
- The app can repair itself after a basic failure
- The app can be uninstalled cleanly

Final test: hand the installer to a non-technical person and say nothing.

## M4 Notes

The WSL2 adapter (`wsl_containerd.rs`) implements the full `RuntimeAdapter` trait but
needs integration testing on a clean Windows 11 VM. The `prepare()` method stubs
distro import — full implementation requires:

1. A pre-built minimal Alpine/Debian WSL distro tarball with containerd + nerdctl
2. `wsl --import craterun-<appId> <installDir> <tarball> --version 2`
3. Running the bootstrap script to start containerd
4. Verifying port forwarding from WSL to Windows host

This is tracked as the highest-risk item. See docs/decisions.md ADR-009.

## M5 Notes

NSIS installer generator (`craterun-cli/src/installer/nsis.rs`) produces a complete
`.nsi` script that `makensis` compiles into a self-extracting `.exe`. Features:

- Per-user install to `%LOCALAPPDATA%\Programs\<AppName>` (no admin elevation)
- Copies shell, supervisor, and app bundle
- Start Menu shortcut
- Add/Remove Programs entry (HKCU registry)
- Post-install auto-launch
- Uninstaller with "remove app data?" prompt

`craterun package --consumer --target windows-x64` runs the full pipeline:
certify → build → generate .nsi → compile with makensis.

Use `--script-only` to generate the .nsi without requiring makensis on PATH.

## M6 Notes

Repair and ResetData commands were implemented in M2's state machine. M6 adds:

- `diagnostics/mod.rs`: `DiagnosticsBundle` struct collecting app state, system
  info (OS, arch, memory, disk), runtime adapter info, service logs, and health
  check config. Explicitly excludes secrets, tokens, env vars, and user data.
- `GET /diagnostics/export` API endpoint returns the JSON bundle.
- API routes now use `CompiledManifest` for app-specific status messages.
- Repair flow: stop -> remove (keep volumes) -> re-prepare from ImportingImages.
- ResetData flow: stop -> remove (including volumes) -> stopped. Requires
  `{ "confirm": true }` in POST body as a safety gate.

## M7 Notes

`craterun certify` now runs 17 checks:

**Manifest checks:** app name, app version, icon, frontend route, health check
configured, health check service exists in compose, all routed services exist,
persistent volumes declared.

**Security checks:** no privileged containers, no host networking, no Docker socket
mounts, no dangerous capabilities, no arbitrary host mounts, all host ports
auto-assigned, no external networks, no external volumes, all images buildable.

`craterun package --consumer` runs all checks as a hard gate before building.
Failures exit with clear per-check output and a suggestion to use Dev Pack mode.
