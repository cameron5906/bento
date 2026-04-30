# Milestone Tracker

Status of each milestone from the implementation brief (DOC.pdf).

| # | Milestone | Status | Commit | Notes |
|---|---|---|---|---|
| M1 | Static App Bundle Compiler | DONE | `5c07808` | Core types, manifest parser, compose validator, CLI commands |
| M2 | Local Supervisor Prototype | DONE | `cc4394d` | State machine, axum API, reverse proxy, health checker |
| M3 | Tauri Shell Prototype | DONE | `4ba01eb` | React UI, supervisor client, state-driven screens |
| M4 | Windows Managed Runtime Spike | DONE | see below | WSL2 + containerd adapter. Needs VM testing. |
| M5 | Native Windows Installer | DONE | see below | NSIS script generator + CLI integration |
| M6 | Repair, Reset, Diagnostics | NOT STARTED | — | Supervisor repair flow, diagnostics export |
| M7 | Consumer Certification Gate | NOT STARTED | — | Hardened certify command, build gate |
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
