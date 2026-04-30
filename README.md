# Bento

**Turn your Docker Compose app into a desktop installer.** Download. Install. Open. No Docker knowledge required.

Bento takes a multi-service containerized application and packages it into a native installer that non-technical users can run — just like installing Chrome. The end user never sees Docker, containers, ports, terminals, or compose files.

## How It Works

**You** (the developer) have a Docker Compose app:

```yaml
# docker-compose.yml
services:
  web:
    build: ./web
    ports: ["3000"]
  api:
    build: ./api
    ports: ["8080"]
  db:
    image: postgres:16
    volumes:
      - db-data:/var/lib/postgresql/data
volumes:
  db-data:
```

**You** write a `bento.yml` that maps it to a desktop app:

```yaml
app:
  id: com.mycompany.myapp
  name: My App
  version: 1.0.0
  icon: ./assets/icon.png

compose:
  file: ./docker-compose.yml

window:
  title: My App
  width: 1200
  height: 800

routes:
  /:
    service: web
    port: 3000
  /api:
    service: api
    port: 8080

health:
  ready:
    service: api
    path: /health
    timeoutSeconds: 120

volumes:
  db-data:
    durability: persistent
```

**You** run one command:

```bash
bento package --consumer
```

**Your user** downloads `MyAppSetup.exe`, double-clicks it, clicks Install, and the app opens in a native desktop window. Their data persists. They can close and reopen it. They can uninstall it from Add/Remove Programs. They never know containers exist.

## Quick Start

```bash
# 1. Initialize from an existing docker-compose.yml
bento init

# 2. Check your environment
bento doctor

# 3. Verify your app is safe for consumer packaging
bento certify

# 4. Build and package
bento package --consumer
```

## What Gets Produced

| Platform | Command | Output |
|---|---|---|
| Windows | `bento package --consumer --target windows-x64` | `MyAppSetup.exe` (NSIS installer) |
| macOS | `bento package --consumer --target macos-arm64` | `MyApp.dmg` (via Tauri) |
| Linux | `bento package --consumer --target linux-x64` | `.deb` + `.AppImage` (via Tauri) |

The installer bundles everything: your app's container images, a local supervisor, and a native desktop shell. The target auto-detects based on your current OS.

## CLI Commands

| Command | What it does |
|---|---|
| `bento init` | Scaffold a `bento.yml` from an existing `docker-compose.yml` |
| `bento doctor` | Check prerequisites (Docker, WSL2, etc.) |
| `bento build` | Build images and produce an app bundle |
| `bento certify` | Run 17 consumer-readiness checks |
| `bento package --consumer` | Full pipeline: certify + build + generate installer |
| `bento run-local` | Build and run locally for development |

## Consumer Safety

Bento enforces strict rules for consumer packaging. These Compose features are **blocked** in consumer mode:

- `privileged: true`
- `network_mode: host`
- Docker socket mounts
- Arbitrary host filesystem mounts
- Fixed host ports
- Dangerous Linux capabilities
- External networks/volumes

If your Compose file uses any of these, `bento certify` will tell you exactly what to fix. Developer-mode packaging (`--mode dev-pack`) exists as an escape hatch.

## Architecture

```
Developer Machine                    End-User Machine
+------------------+                +------------------+
|   bento CLI      |                | Native Installer |
|                  |   produces     |                  |
| - reads bento.yml| ============> | - copies files   |
| - builds images  |  Setup.exe    | - registers app  |
| - exports OCI    |  .dmg         | - launches shell |
| - generates      |  .deb         +--------+---------+
|   installer      |                        |
+------------------+                        v
                                   +------------------+
                                   | Desktop Shell    |
                                   | (Tauri + React)  |
                                   |                  |
                                   | - loading screen |
                                   | - webview window |
                                   | - error recovery |
                                   +--------+---------+
                                            |
                                            v
                                   +------------------+
                                   | Local Supervisor |
                                   |                  |
                                   | - state machine  |
                                   | - reverse proxy  |
                                   | - health checks  |
                                   | - container mgmt |
                                   +--------+---------+
                                            |
                                            v
                                   +------------------+
                                   | Container Runtime|
                                   |                  |
                                   | web + api + db   |
                                   +------------------+
```

## Supported App Shape

Bento works best with apps that have:
- A web frontend service
- A backend/API service
- An optional database with persistent volumes
- An HTTP health check endpoint

## Requirements

**Developer machine** (for packaging):
- Docker (for building and exporting images)
- Rust toolchain (for building Bento itself)
- NSIS (Windows installer compilation) or Tauri CLI (macOS/Linux)

**End-user machine:**
- Windows 11 x64 with Docker Desktop or WSL2
- macOS with Docker Desktop or OrbStack
- Linux with Docker Engine

## Project Structure

```
crates/
  bento-core/          # Shared types, state machine, error translation
  bento-bundle/        # Manifest parser, Compose validator, bundle I/O
  bento-runtime/       # Container runtime adapter trait + implementations
  bento-supervisor/    # Local supervisor with HTTP API and reverse proxy
  bento-cli/           # Developer CLI
apps/
  shell-tauri/         # Tauri v2 + React desktop shell
examples/
  hello-web-api/       # Sample app (Node web + API + Postgres)
```

## License

MIT
