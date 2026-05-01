const express = require("express");
const multer = require("multer");
const { Pool } = require("pg");
const Redis = require("ioredis");
const cors = require("cors");
const { v4: uuidv4 } = require("uuid");
const sharp = require("sharp");
const path = require("path");
const fs = require("fs");

const app = express();
const PORT = 8080;
const PHOTO_DIR = process.env.PHOTO_STORAGE_PATH || "/data/photos";

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  connectionTimeoutMillis: 5000,
  idleTimeoutMillis: 30000,
});
pool.on("error", (err) => console.error("Postgres pool error:", err.message));

const redis = new Redis(process.env.REDIS_URL || "redis://redis:6379");

app.use(cors());
app.use(express.json());

fs.mkdirSync(path.join(PHOTO_DIR, "originals"), { recursive: true });
fs.mkdirSync(path.join(PHOTO_DIR, "thumbnails"), { recursive: true });

const upload = multer({
  storage: multer.diskStorage({
    destination: path.join(PHOTO_DIR, "originals"),
    filename: (req, file, cb) => {
      const id = uuidv4();
      const ext = path.extname(file.originalname) || ".jpg";
      cb(null, `${id}${ext}`);
    },
  }),
  limits: { fileSize: 50 * 1024 * 1024 },
  fileFilter: (req, file, cb) => {
    const allowed = /\.(jpe?g|png|gif|webp|bmp|tiff?)$/i;
    cb(null, allowed.test(path.extname(file.originalname)));
  },
});

async function ensureSchema() {
  await pool.query("CREATE EXTENSION IF NOT EXISTS vector");
  await pool.query(`
    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    )
  `);
  await pool.query(`
    CREATE TABLE IF NOT EXISTS photos (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      filename TEXT NOT NULL,
      original_name TEXT,
      size_bytes INTEGER,
      width INTEGER,
      height INTEGER,
      description TEXT,
      tags TEXT[] DEFAULT '{}',
      status TEXT DEFAULT 'pending',
      embedding vector(1536),
      created_at TIMESTAMPTZ DEFAULT NOW(),
      analyzed_at TIMESTAMPTZ
    )
  `);
  await pool.query(`
    CREATE INDEX IF NOT EXISTS idx_photos_embedding
    ON photos USING hnsw (embedding vector_cosine_ops)
  `);
}

ensureSchema().catch((e) => console.error("Schema init:", e.message));

// --- Health ---
app.get("/health", async (req, res) => {
  try {
    await pool.query("SELECT 1");
    res.json({ status: "healthy", database: "connected" });
  } catch (e) {
    res.status(503).json({ status: "unhealthy", error: e.message });
  }
});

// --- Settings (API key management) ---
app.get("/settings", async (req, res) => {
  const result = await pool.query("SELECT key, value FROM settings");
  const settings = {};
  for (const row of result.rows) {
    // Mask the API key for display
    if (row.key === "openai_api_key" && row.value) {
      settings[row.key] = row.value.slice(0, 7) + "..." + row.value.slice(-4);
      settings["openai_api_key_set"] = true;
    } else {
      settings[row.key] = row.value;
    }
  }
  res.json(settings);
});

app.post("/settings", async (req, res) => {
  const { key, value } = req.body;
  if (!key || !value) return res.status(400).json({ error: "key and value required" });
  await pool.query(
    "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2",
    [key, value]
  );
  res.json({ ok: true });
});

// --- Upload ---
app.post("/photos/upload", upload.array("photos", 20), async (req, res) => {
  const results = [];

  for (const file of req.files) {
    // Generate thumbnail
    const thumbPath = path.join(PHOTO_DIR, "thumbnails", file.filename);
    try {
      const metadata = await sharp(file.path).metadata();
      await sharp(file.path)
        .resize(400, 400, { fit: "cover" })
        .jpeg({ quality: 80 })
        .toFile(thumbPath);

      const result = await pool.query(
        `INSERT INTO photos (filename, original_name, size_bytes, width, height, status)
         VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING id, filename, status, created_at`,
        [file.filename, file.originalname, file.size, metadata.width, metadata.height]
      );

      const photo = result.rows[0];
      // Enqueue analysis job
      await redis.lpush("analysis_queue", JSON.stringify({ photoId: photo.id, filename: file.filename }));

      results.push(photo);
    } catch (e) {
      console.error("Upload processing error:", e.message);
    }
  }

  res.json({ uploaded: results.length, photos: results });
});

// --- List photos ---
app.get("/photos", async (req, res) => {
  const result = await pool.query(
    "SELECT id, filename, original_name, description, tags, status, width, height, created_at FROM photos ORDER BY created_at DESC LIMIT 200"
  );
  res.json(result.rows);
});

// --- Get single photo ---
app.get("/photos/:id", async (req, res) => {
  const result = await pool.query(
    "SELECT id, filename, original_name, description, tags, status, width, height, size_bytes, created_at, analyzed_at FROM photos WHERE id = $1",
    [req.params.id]
  );
  if (result.rows.length === 0) return res.status(404).json({ error: "not found" });
  res.json(result.rows[0]);
});

// --- Semantic search ---
app.get("/photos/search/:query", async (req, res) => {
  // Get OpenAI API key from settings
  const keyResult = await pool.query("SELECT value FROM settings WHERE key = 'openai_api_key'");
  if (keyResult.rows.length === 0) {
    return res.status(400).json({ error: "OpenAI API key not configured" });
  }

  const apiKey = keyResult.rows[0].value;
  const query = req.params.query;

  try {
    // Generate embedding for the search query
    const embResponse = await fetch("https://api.openai.com/v1/embeddings", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: "text-embedding-3-small",
        input: query,
      }),
    });

    const embData = await embResponse.json();
    if (!embResponse.ok) {
      return res.status(500).json({ error: embData.error?.message || "embedding failed" });
    }

    const queryVector = JSON.stringify(embData.data[0].embedding);

    // Cosine similarity search via pgvector
    const result = await pool.query(
      `SELECT id, filename, original_name, description, tags, status, width, height,
              1 - (embedding <=> $1::vector) AS similarity
       FROM photos
       WHERE embedding IS NOT NULL
       ORDER BY embedding <=> $1::vector
       LIMIT 20`,
      [queryVector]
    );

    res.json(result.rows);
  } catch (e) {
    res.status(500).json({ error: e.message });
  }
});

// --- Serve images ---
app.get("/photos/file/:filename", (req, res) => {
  const filePath = path.join(PHOTO_DIR, "originals", req.params.filename);
  if (!fs.existsSync(filePath)) return res.status(404).send("not found");
  res.sendFile(filePath);
});

app.get("/photos/thumb/:filename", (req, res) => {
  const filePath = path.join(PHOTO_DIR, "thumbnails", req.params.filename);
  if (!fs.existsSync(filePath)) return res.status(404).send("not found");
  res.sendFile(filePath);
});

app.listen(PORT, () => console.log(`Bento Photos API on port ${PORT}`));
