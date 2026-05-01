const { Pool } = require("pg");
const Redis = require("ioredis");
const OpenAI = require("openai");
const fs = require("fs");
const path = require("path");

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  connectionTimeoutMillis: 5000,
});
pool.on("error", (err) => console.error("Postgres pool error:", err.message));

const redis = new Redis(process.env.REDIS_URL || "redis://redis:6379");

const PHOTO_DIR = "/data/photos/originals";
const POLL_INTERVAL = 2000;

async function getApiKey() {
  const result = await pool.query(
    "SELECT value FROM settings WHERE key = 'openai_api_key'"
  );
  return result.rows[0]?.value || null;
}

async function analyzePhoto(photoId, filename) {
  const apiKey = await getApiKey();
  if (!apiKey) {
    console.log(`No API key configured — skipping analysis for ${photoId}`);
    await pool.query(
      "UPDATE photos SET status = 'waiting_for_key' WHERE id = $1",
      [photoId]
    );
    // Re-queue for later
    await redis.lpush(
      "analysis_queue",
      JSON.stringify({ photoId, filename })
    );
    // Wait before retrying to avoid tight loop
    await sleep(10000);
    return;
  }

  const openai = new OpenAI({ apiKey });

  console.log(`Analyzing photo ${photoId} (${filename})`);
  await pool.query("UPDATE photos SET status = 'analyzing' WHERE id = $1", [
    photoId,
  ]);

  const imagePath = path.join(PHOTO_DIR, filename);
  if (!fs.existsSync(imagePath)) {
    console.error(`Image file not found: ${imagePath}`);
    await pool.query(
      "UPDATE photos SET status = 'error' WHERE id = $1",
      [photoId]
    );
    return;
  }

  const imageBase64 = fs.readFileSync(imagePath).toString("base64");
  const mimeType = filename.match(/\.png$/i) ? "image/png" : "image/jpeg";

  try {
    // Step 1: Vision analysis — describe the photo and generate tags
    const visionResponse = await openai.chat.completions.create({
      model: "gpt-4.1-mini",
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: `Analyze this photo. Respond in JSON with exactly this format:
{
  "description": "A detailed 1-2 sentence description of what's in this photo",
  "tags": ["tag1", "tag2", "tag3", "tag4", "tag5"]
}
Provide 3-8 descriptive tags covering subjects, setting, mood, colors, and activities visible.`,
            },
            {
              type: "image_url",
              image_url: {
                url: `data:${mimeType};base64,${imageBase64}`,
                detail: "low",
              },
            },
          ],
        },
      ],
      response_format: { type: "json_object" },
      max_tokens: 300,
    });

    const analysisText = visionResponse.choices[0].message.content;
    let analysis;
    try {
      analysis = JSON.parse(analysisText);
    } catch {
      analysis = { description: analysisText, tags: [] };
    }

    const description = analysis.description || "No description available";
    const tags = Array.isArray(analysis.tags) ? analysis.tags : [];

    console.log(`  Description: ${description}`);
    console.log(`  Tags: ${tags.join(", ")}`);

    // Step 2: Generate embedding from the description for semantic search
    const embeddingResponse = await openai.embeddings.create({
      model: "text-embedding-3-small",
      input: `${description}. Tags: ${tags.join(", ")}`,
    });

    const embedding = embeddingResponse.data[0].embedding;
    const vectorStr = JSON.stringify(embedding);

    // Step 3: Store results in Postgres
    await pool.query(
      `UPDATE photos
       SET description = $1,
           tags = $2,
           embedding = $3::vector,
           status = 'complete',
           analyzed_at = NOW()
       WHERE id = $4`,
      [description, tags, vectorStr, photoId]
    );

    console.log(`  Analysis complete for ${photoId}`);
  } catch (e) {
    console.error(`  Analysis failed for ${photoId}:`, e.message);
    await pool.query(
      "UPDATE photos SET status = 'error', description = $1 WHERE id = $2",
      [e.message, photoId]
    );
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function processQueue() {
  console.log("Bento Photos Worker started — waiting for jobs...");

  while (true) {
    try {
      // Blocking pop with 5-second timeout
      const result = await redis.brpop("analysis_queue", 5);
      if (result) {
        const job = JSON.parse(result[1]);
        await analyzePhoto(job.photoId, job.filename);
      }
    } catch (e) {
      console.error("Queue processing error:", e.message);
      await sleep(POLL_INTERVAL);
    }
  }
}

processQueue();
