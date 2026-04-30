const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 3000;

const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Hello Web API</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: linear-gradient(135deg, #0f0f0f, #1a1a2e);
      color: #e0e0e0;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .card {
      background: #1e1e2e;
      border: 1px solid #333;
      border-radius: 12px;
      padding: 48px;
      max-width: 500px;
      text-align: center;
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    }
    h1 { font-size: 28px; margin-bottom: 12px; color: #fff; }
    p { color: #999; margin-bottom: 24px; line-height: 1.6; }
    .status { font-size: 14px; color: #666; }
    .status.ok { color: #22c55e; }
    .status.err { color: #ef4444; }
    button {
      padding: 10px 24px;
      background: #6366f1;
      color: #fff;
      border: none;
      border-radius: 6px;
      font-size: 14px;
      cursor: pointer;
      margin-bottom: 16px;
    }
    button:hover { background: #4f46e5; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Hello Web API</h1>
    <p>This app was packaged with CrateRun.<br>
    It runs a web frontend, a Node API, and a Postgres database — all inside containers.</p>
    <button onclick="checkHealth()">Check API Health</button>
    <div id="status" class="status">Click the button to check the API.</div>
  </div>
  <script>
    async function checkHealth() {
      const el = document.getElementById('status');
      el.className = 'status';
      el.textContent = 'Checking...';
      try {
        const res = await fetch('/api/health');
        const data = await res.json();
        el.className = 'status ok';
        el.textContent = 'API is healthy: ' + JSON.stringify(data);
      } catch (e) {
        el.className = 'status err';
        el.textContent = 'API unreachable: ' + e.message;
      }
    }
  </script>
</body>
</html>`;

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/html" });
  res.end(html);
});

server.listen(PORT, () => {
  console.log("Web server listening on port " + PORT);
});
