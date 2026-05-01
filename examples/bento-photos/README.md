# Bento Photos

A local Google Photos alternative with AI-powered image analysis and semantic search — packaged as a native desktop app by Bento.

## What It Does

Upload photos, and an AI worker automatically analyzes each one using OpenAI's Vision API — generating descriptions, tags, and vector embeddings. Search your photos with natural language ("sunset over mountains") and find matches ranked by semantic similarity.

Everything runs locally in containers. Your photos and API key never leave your machine.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Bento Shell                     │
│             (Tauri native window)                │
└────────────────────┬────────────────────────────┘
                     │ webview
┌────────────────────▼────────────────────────────┐
│                 Web (React)                      │
│  • Masonry photo grid                            │
│  • Drag-and-drop upload                          │
│  • Semantic search bar                           │
│  • Settings screen (API key)                     │
│  • Photo detail modal                            │
│                                          :3000   │
└────────────────────┬────────────────────────────┘
                     │ /api/*
┌────────────────────▼────────────────────────────┐
│                API (Express)                     │
│  • POST /photos/upload → save + enqueue job      │
│  • GET  /photos → list all                       │
│  • GET  /photos/search/:q → embed query,         │
│         cosine similarity via pgvector            │
│  • POST /settings → store API key                │
│  • GET  /health → DB connectivity check          │
│                                          :8080   │
└──────┬─────────────────────────────┬────────────┘
       │ enqueue                     │ query
┌──────▼──────┐            ┌────────▼─────────────┐
│    Redis    │            │   PostgreSQL + pgvector│
│  job queue  │            │                        │
│             │            │  • photos table         │
│  :6379      │            │  • vector(1536) column  │
└──────┬──────┘            │  • HNSW cosine index   │
       │ consume           │  • settings table       │
┌──────▼──────────────┐    │                :5432   │
│   Worker (Node.js)  │    └────────────────────────┘
│                     │              ▲
│  1. Pick job from   │              │
│     Redis queue     │              │
│  2. Send photo to   │    store results
│     GPT-4.1 Mini    ├──────────────┘
│     Vision API      │
│  3. Get description  │
│     + tags           │
│  4. Generate vector  │
│     embedding via    │
│     text-embedding-  │
│     3-small          │
│  5. Store in DB      │
└─────────────────────┘
```

## Services

| Service | Tech | Role |
|---|---|---|
| **web** | Node.js + static HTML/CSS/JS | Photo gallery UI with search, upload, settings |
| **api** | Node.js + Express + Sharp | REST API, thumbnail generation, semantic search via pgvector |
| **worker** | Node.js + OpenAI SDK | Background AI analysis: vision descriptions, tag generation, embedding creation |
| **db** | PostgreSQL 17 + pgvector | Photo metadata, AI descriptions, 1536-dim vector embeddings with HNSW index |
| **redis** | Redis 7 | Job queue between API (producer) and worker (consumer) |

## How It Uses Bento

This app demonstrates key Bento capabilities:

- **5-service orchestration** — Web, API, Worker, Database, and Redis all managed by a single installer
- **Worker service pattern** — The background worker processes AI jobs asynchronously, showing Bento handles more than just frontend+backend
- **Persistent volumes** — Both `db-data` (Postgres with embeddings) and `photo-storage` (uploaded images) survive app restarts
- **Route mapping** — `/` routes to the web UI, `/api/*` routes to the Express backend through Bento's reverse proxy
- **Health checks** — Supervisor waits for the API's `/health` endpoint before showing the app
- **Consumer safety** — All 17 certification checks pass (no privileged containers, no host ports, no socket mounts)

## Bento Configuration

```yaml
# bento.yml
routes:
  /:
    service: web
    port: 3000
  /api:
    service: api
    port: 8080

volumes:
  db-data:
    durability: persistent
  photo-storage:
    durability: persistent
```

## Running Locally (Development)

```bash
# With Docker Compose directly
docker compose up --build

# With Bento CLI
bento run-local

# Package as a Windows installer
bento package --consumer --target windows-x64
```

## First Launch

1. The app opens to the **Settings** screen
2. Enter your OpenAI API key (stored locally in the database, never sent anywhere except OpenAI)
3. Go back to the gallery and upload photos
4. Watch as the worker analyzes each photo in the background
5. Search with natural language once analysis is complete
