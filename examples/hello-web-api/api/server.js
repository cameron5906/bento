const http = require("http");
const { Pool } = require("pg");

const PORT = 8080;
const DATABASE_URL = process.env.DATABASE_URL || "postgres://postgres:postgres@db:5432/app";

const pool = new Pool({
  connectionString: DATABASE_URL,
  // Retry connections — the db container may still be starting
  connectionTimeoutMillis: 5000,
  idleTimeoutMillis: 30000,
});

// Prevent unhandled pool errors from crashing the process
pool.on("error", (err) => {
  console.error("Postgres pool error (non-fatal):", err.message);
});

async function ensureSchema() {
  await pool.query(`
    CREATE TABLE IF NOT EXISTS app_counter (
      id INTEGER PRIMARY KEY DEFAULT 1,
      value INTEGER NOT NULL DEFAULT 0,
      updated_at TIMESTAMPTZ DEFAULT NOW()
    )
  `);
  await pool.query(`
    INSERT INTO app_counter (id, value) VALUES (1, 0)
    ON CONFLICT (id) DO NOTHING
  `);
}

ensureSchema().catch(err => {
  console.error("Schema init failed (will retry on first request):", err.message);
});

const server = http.createServer(async (req, res) => {
  res.setHeader("Content-Type", "application/json");

  try {
    if (req.url === "/health") {
      const result = await pool.query("SELECT 1 as ok");
      res.writeHead(200);
      res.end(JSON.stringify({
        status: "healthy",
        database: "connected",
        timestamp: new Date().toISOString(),
      }));
      return;
    }

    if (req.url === "/count" && req.method === "GET") {
      await ensureSchema();
      const result = await pool.query("SELECT value FROM app_counter WHERE id = 1");
      res.writeHead(200);
      res.end(JSON.stringify({ value: result.rows[0]?.value ?? 0 }));
      return;
    }

    if (req.url === "/count/increment" && req.method === "POST") {
      await ensureSchema();
      const result = await pool.query(
        "UPDATE app_counter SET value = value + 1, updated_at = NOW() WHERE id = 1 RETURNING value"
      );
      res.writeHead(200);
      res.end(JSON.stringify({ value: result.rows[0].value }));
      return;
    }

    if (req.url === "/count/decrement" && req.method === "POST") {
      await ensureSchema();
      const result = await pool.query(
        "UPDATE app_counter SET value = value - 1, updated_at = NOW() WHERE id = 1 RETURNING value"
      );
      res.writeHead(200);
      res.end(JSON.stringify({ value: result.rows[0].value }));
      return;
    }

    res.writeHead(404);
    res.end(JSON.stringify({ error: "not found" }));
  } catch (e) {
    res.writeHead(500);
    res.end(JSON.stringify({ error: e.message }));
  }
});

server.listen(PORT, () => {
  console.log("API server listening on port " + PORT);
});
