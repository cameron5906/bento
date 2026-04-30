# Milestone Tracker

Status of each milestone from the implementation brief (DOC.pdf).

| # | Milestone | Status | Commit | Notes |
|---|---|---|---|---|
| M1 | Static App Bundle Compiler | DONE | `5c07808` | Core types, manifest parser, compose validator, CLI commands |
| M2 | Local Supervisor Prototype | DONE | `cc4394d` | State machine, axum API, reverse proxy, health checker |
| M3 | Tauri Shell Prototype | DONE | `4ba01eb` | React UI, supervisor client, state-driven screens |
| M4 | Windows Managed Runtime Spike | IN PROGRESS | — | WSL2 + containerd adapter. Riskiest milestone. |
| M5 | Native Windows Installer | NOT STARTED | — | NSIS-based installer generation |
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
