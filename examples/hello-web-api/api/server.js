const http = require("http");
const { Pool } = require("pg");

const PORT = 8080;
const DATABASE_URL = process.env.DATABASE_URL || "postgres://postgres:postgres@db:5432/app";

const pool = new Pool({ connectionString: DATABASE_URL });

const server = http.createServer(async (req, res) => {
  res.setHeader("Content-Type", "application/json");

  if (req.url === "/health") {
    try {
      const result = await pool.query("SELECT 1 as ok");
      res.writeHead(200);
      res.end(JSON.stringify({
        status: "healthy",
        database: "connected",
        timestamp: new Date().toISOString(),
      }));
    } catch (e) {
      res.writeHead(503);
      res.end(JSON.stringify({
        status: "unhealthy",
        database: "disconnected",
        error: e.message,
      }));
    }
    return;
  }

  if (req.url === "/items" && req.method === "GET") {
    try {
      await pool.query(`
        CREATE TABLE IF NOT EXISTS items (
          id SERIAL PRIMARY KEY,
          name TEXT NOT NULL,
          created_at TIMESTAMPTZ DEFAULT NOW()
        )
      `);
      const result = await pool.query("SELECT * FROM items ORDER BY created_at DESC LIMIT 50");
      res.writeHead(200);
      res.end(JSON.stringify(result.rows));
    } catch (e) {
      res.writeHead(500);
      res.end(JSON.stringify({ error: e.message }));
    }
    return;
  }

  res.writeHead(404);
  res.end(JSON.stringify({ error: "not found" }));
});

server.listen(PORT, () => {
  console.log("API server listening on port " + PORT);
});
