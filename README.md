<p align="center">
  <img src="assets/logo.png" alt="Bento" width="200" />
</p>

<h1 align="center">Bento</h1>

<p align="center">
  <strong>Turn your Docker Compose app into a desktop installer.</strong><br>
  Download. Install. Open. No Docker knowledge required.
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#how-it-works">How It Works</a> &bull;
  <a href="#examples">Examples</a> &bull;
  <a href="#cli-commands">CLI</a> &bull;
  <a href="#architecture">Architecture</a>
</p>

---

## The Problem

You built a great app with Docker Compose — a web frontend, an API, a database, maybe a worker. It runs perfectly on your machine.

Now ship it to someone who doesn't know what Docker is.

**Without Bento**, that means: install Docker Desktop, clone the repo, open a terminal, run `docker compose up`, find the right port, open a browser, pray nothing breaks. Your user gave up at step one.

**With Bento**, that means: download the installer, double-click, click Install, the app opens. That's it. They never see a terminal. They don't know containers exist.

## How It Works

**You** have a Docker Compose app. **You** add a `bento.yml`:

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

splash:
  logo: ./assets/splash.png
  messages:
    - "Loading your app..."
    - "Almost there..."
```

**You** run one command:

```
bento box
```

**Out comes an installer.** Your user downloads it, double-clicks, and the app opens in a native desktop window with a splash screen, progress bar, and zero terminal output.

## Quick Start

```bash
# In your project directory (has docker-compose.yml)
bento init        # generates bento.yml from your compose file
bento doctor      # checks Docker, WSL, etc.
bento certify     # 18 consumer safety checks
bento box         # → produces MyAppSetup.exe / .dmg / .deb
```

## What Gets Produced

| Platform | Output | How |
|---|---|---|
| **Windows** | `MyAppSetup.exe` | NSIS installer with Start Menu shortcut |
| **macOS** | `MyApp.dmg` | Tauri native app bundle |
| **Linux** | `.deb` + `.AppImage` | Tauri native packages |

Target auto-detects from your OS. Override with `--target windows-x64`, `--target macos-arm64`, etc.

## What Users See

1. **Install** — one click, no admin needed, no Docker install
2. **Splash screen** — your logo, your loading messages, progress bar
3. **Your app** — opens in a native desktop window, full-screen capable
4. **Close & reopen** — instant restart, data persists, containers stay warm for 15 minutes
5. **Uninstall** — clean removal from Add/Remove Programs, optional data wipe

What users **never** see: Docker, Compose, WSL, terminals, ports, container logs, `localhost:3000`.

## Examples

### Bento Photos

A local Google Photos alternative with AI-powered analysis — 5 services packaged into a single installer.

| Service | Role |
|---|---|
| React web UI | Masonry photo grid, drag-and-drop upload, semantic search |
| Node.js API | REST endpoints, thumbnail generation, pgvector search |
| Node.js worker | Background AI analysis via OpenAI Vision + embeddings |
| PostgreSQL + pgvector | Photo metadata + 1536-dim vector search |
| Redis | Async job queue between API and worker |

Upload photos → AI generates descriptions and tags → search "sunset over mountains" → finds matches via cosine similarity. All running locally from a double-click installer.

```bash
cd examples/bento-photos
bento box    # → BentoPhotosSetup.exe (430 MB)
```

### Hello Web API

A simpler 3-service example: Node.js web + API + Postgres with a persistent counter.

```bash
cd examples/hello-web-api
bento box    # → HelloWebAPISetup.exe (267 MB)
```

## CLI Commands

| Command | What it does |
|---|---|
| `bento init` | Generate `bento.yml` from an existing compose file |
| `bento doctor` | Check prerequisites (Docker, WSL2, etc.) |
| `bento build` | Build images and produce an app bundle |
| `bento certify` | Run 18 consumer-readiness checks |
| `bento box` | Full pipeline: certify → build → package installer |
| `bento run-local` | Build and run locally for development |

## Consumer Safety

Bento enforces strict rules for consumer packaging. Your app **cannot** use:

- `privileged: true`
- `network_mode: host`
- Docker socket mounts
- Arbitrary host filesystem mounts
- Fixed host ports
- Dangerous Linux capabilities
- External networks/volumes

`bento certify` tells you exactly what to fix. `bento build` is available as an escape hatch for apps that need these features but aren't consumer-facing.

## Customization

### Splash Screen

Show your brand during loading with custom logos and rotating messages:

```yaml
splash:
  logo: ./assets/my-splash.png
  messages:
    - "Preparing your workspace..."
    - "Loading modules..."
    - "Almost ready..."
```

If you don't provide custom config, Bento shows its default logo with fun loading messages.

### App Icons

Provide a PNG (ideally 1024x1024) and Bento generates platform-appropriate formats:

```yaml
app:
  icon: ./assets/icon.png   # → .ico (Windows), .icns (macOS), sized PNGs (Linux)
```

### Persistent Data

Declare which volumes survive app restarts and which are disposable:

```yaml
volumes:
  db-data:
    durability: persistent   # kept across restarts and updates
    backup: true
  cache:
    durability: disposable   # cleared on repair/reset
```

## Architecture

```
Developer Machine                     End-User Machine
┌──────────────────┐                  ┌──────────────────┐
│   bento CLI      │    produces      │  Native Window   │
│                  │ ════════════════>│  (Tauri + React) │
│  bento.yml       │  .exe / .dmg    │                  │
│  + compose.yml   │  / .deb         │  Splash screen   │
│  → bento box     │                 │  Loading states  │
└──────────────────┘                  │  Error recovery  │
                                      └────────┬─────────┘
                                               │
                                      ┌────────▼─────────┐
                                      │   Supervisor     │
                                      │                  │
                                      │  State machine   │
                                      │  Reverse proxy   │
                                      │  Health checks   │
                                      │  15-min idle     │
                                      └────────┬─────────┘
                                               │
                                      ┌────────▼─────────┐
                                      │   Containers     │
                                      │                  │
                                      │  web + api + db  │
                                      │  + worker + ...  │
                                      └──────────────────┘
```

## Project Structure

```
crates/
  bento-core/          Shared types, state machine, error translation
  bento-bundle/        Manifest parser, Compose validator, bundle I/O
  bento-runtime/       Container runtime adapters (Docker, WSL2)
  bento-supervisor/    Local supervisor with HTTP API and reverse proxy
  bento-cli/           Developer CLI (bento box, init, doctor, certify)
apps/
  shell-tauri/         Tauri v2 + React desktop shell
examples/
  bento-photos/        5-service AI photo gallery demo
  hello-web-api/       3-service counter app demo
```

## Requirements

**Developer machine** (for packaging):
- Docker (for building and exporting images)
- Rust toolchain
- NSIS (Windows) or Tauri CLI (macOS/Linux)

**End-user machine:**
- Windows 11 with Docker Desktop
- macOS with Docker Desktop or OrbStack
- Linux with Docker Engine

## License

MIT
