//! Per-model pricing. Approximate list prices, per 1M tokens, USD.
//! Prefix-matched, longest prefix wins. Match also works on OpenRouter-style
//! slugs ("anthropic/claude-opus-..." -> "claude-opus-...").

#[derive(Debug, Clone, Copy, Default)]
pub struct Pricing {
    /// USD per 1M tokens
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// USD per hour of wall-clock usage (rented GPU / self-hosted inference)
    pub per_hour: f64,
}

#[derive(Debug, Clone)]
pub struct PriceEntry {
    pub prefix: String,
    /// where this row came from: "curated" | "embedded" | "cache" | "custom"
    pub src: String,
    pub price: Pricing,
}

pub struct PriceDB {
    entries: Vec<PriceEntry>,
    pub embedded_count: usize,
    pub cache_count: usize,
    pub custom_count: usize,
    pub embedded_fetched_at_ms: u64,
}

/// The bundled price database: a full OpenRouter catalog snapshot, refreshed
/// monthly by CI and committed to `data/prices.json`. Ships inside the binary —
/// every install knows every model's price with zero network, zero scanning.
const EMBEDDED: &str = include_str!("../data/prices.json");

fn prices_from_doc(v: &serde_json::Value, src: &str, db: &mut PriceDB) -> (usize, u64) {
    let fetched = v
        .get("fetched_at_ms")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let mut n = 0usize;
    if let Some(list) = v.get("models").and_then(serde_json::Value::as_array) {
        for m in list {
            let id = match m.get("id").and_then(serde_json::Value::as_str) {
                Some(s) => s,
                None => continue,
            };
            let get = |k: &str| match m.get(k) {
                Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
                Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0.0),
                _ => 0.0,
            };
            db.entries.push(PriceEntry {
                prefix: id.to_string(),
                src: src.to_string(),
                price: Pricing {
                    input: get("prompt"),
                    output: get("completion"),
                    cache_read: get("cache_read"),
                    cache_write: get("cache_write"),
                    per_hour: 0.0,
                },
            });
            n += 1;
        }
    }
    (n, fetched)
}

/// (prefix, input, output, cache_read, cache_write) — curated list-price table.
/// Sources: vendor price pages as of mid-2026. Treat as approximate.
const TABLE: &[(&str, f64, f64, f64, f64)] = &[
    // --- Anthropic ---
    ("claude-opus-4", 15.0, 75.0, 1.5, 18.75),
    ("claude-sonnet-4", 3.0, 15.0, 0.3, 3.75),
    ("claude-haiku-4", 1.0, 5.0, 0.1, 1.25),
    ("claude-3-7-sonnet", 3.0, 15.0, 0.3, 3.75),
    ("claude-3-5-sonnet", 3.0, 15.0, 0.3, 3.75),
    ("claude-3-5-haiku", 0.8, 4.0, 0.08, 1.0),
    ("claude-3-opus", 15.0, 75.0, 1.5, 18.75),
    // --- OpenAI ---
    ("gpt-5", 1.25, 10.0, 0.125, 1.25),
    ("gpt-4.1", 2.0, 8.0, 0.5, 2.0),
    ("gpt-4o", 2.5, 10.0, 1.25, 2.5),
    ("o3", 2.0, 8.0, 0.5, 2.0),
    ("o4-mini", 1.1, 4.4, 0.28, 1.1),
    ("codex", 1.25, 10.0, 0.125, 1.25),
    ("chatgpt", 5.0, 20.0, 0.5, 5.0),
    // --- Google ---
    ("gemini-3", 2.0, 12.0, 0.2, 2.0),
    ("gemini-2", 1.25, 10.0, 0.31, 1.25),
    ("gemini", 1.25, 10.0, 0.31, 1.25),
    // --- Zhipu / GLM ---
    ("glm-5", 1.0, 3.2, 0.11, 1.0),
    ("glm-4.6", 0.6, 2.2, 0.11, 0.6),
    ("glm-4", 0.6, 2.2, 0.11, 0.6),
    // --- DeepSeek ---
    ("deepseek-chat", 0.27, 1.1, 0.07, 0.27),
    ("deepseek-reasoner", 0.55, 2.19, 0.14, 0.55),
    ("deepseek", 0.27, 1.1, 0.07, 0.27),
    // --- Alibaba ---
    ("qwen3", 0.4, 1.2, 0.04, 0.4),
    ("qwen", 0.4, 1.2, 0.04, 0.4),
    // --- Moonshot ---
    ("kimi-k2", 0.6, 2.5, 0.07, 0.6),
    ("kimi", 0.6, 2.5, 0.07, 0.6),
    // --- xAI ---
    ("grok-4", 3.0, 15.0, 0.3, 3.0),
    ("grok-3", 3.0, 15.0, 0.3, 3.0),
    ("grok", 3.0, 15.0, 0.3, 3.0),
    // --- Meta ---
    ("llama-4", 0.5, 0.8, 0.05, 0.5),
    ("llama-3", 0.9, 0.9, 0.09, 0.9),
    ("llama", 0.3, 0.3, 0.03, 0.3),
    // --- Mistral ---
    ("mistral-large", 2.0, 6.0, 0.2, 2.0),
    ("mistral-small", 0.1, 0.3, 0.01, 0.1),
    ("devstral", 0.1, 0.3, 0.01, 0.1),
    ("codestral", 0.3, 0.9, 0.03, 0.3),
    ("mistral", 0.15, 0.6, 0.015, 0.15),
    // --- Others ---
    ("minimax", 0.3, 1.2, 0.03, 0.3),
    ("command", 2.5, 10.0, 0.25, 2.5),
    ("phi", 0.25, 1.0, 0.03, 0.25),
];

impl PriceDB {
    /// Curated vendor list + the full embedded catalog — the "everything" DB.
    pub fn builtin() -> Self {
        let mut db = Self::curated();
        let (n, fetched) = prices_from_doc(
            &serde_json::from_str(EMBEDDED).expect("embedded data/prices.json must parse"),
            "embedded",
            &mut db,
        );
        db.embedded_count = n;
        db.embedded_fetched_at_ms = fetched;
        db
    }

    /// Just the curated list-price table (used by tests / lean paths).
    pub fn curated() -> Self {
        let entries = TABLE
            .iter()
            .map(|(prefix, i, o, cr, cw)| PriceEntry {
                prefix: prefix.to_string(),
                src: "curated".to_string(),
                price: Pricing {
                    input: *i,
                    output: *o,
                    cache_read: *cr,
                    cache_write: *cw,
                    per_hour: 0.0,
                },
            })
            .collect();
        PriceDB {
            entries,
            embedded_count: 0,
            cache_count: 0,
            custom_count: 0,
            embedded_fetched_at_ms: 0,
        }
    }

    /// `src` tags the origin and doubles as precedence: on equal-length
    /// prefix ties the LATEST entry wins, so the load order
    /// curated → embedded → cache → custom is also the override order.
    pub fn push(&mut self, prefix: &str, src: &str, price: Pricing) {
        self.entries.push(PriceEntry {
            prefix: prefix.to_string(),
            src: src.to_string(),
            price,
        });
    }

    pub fn entries(&self) -> &[PriceEntry] {
        &self.entries
    }

    fn key_of(model: &str) -> &str {
        // "anthropic/claude-opus-4" -> "claude-opus-4" so OpenRouter slugs match
        model.rsplit('/').next().unwrap_or(model)
    }

    pub fn lookup(&self, model: &str) -> Option<Pricing> {
        // normalize: OpenRouter uses dots ("claude-fable-5.1"), lmarena/agents
        // use hyphens — treat both separators as equivalent
        let norm = |s: &str| {
            s.to_lowercase()
                .chars()
                .map(|c| {
                    if c == '.' || c == '_' || c == '-' {
                        '-'
                    } else {
                        c
                    }
                })
                .collect::<String>()
        };
        let full = norm(model);
        let seg = Self::key_of(&full).to_string();

        let mut best: Option<(usize, Pricing)> = None; // (prefix_len, price)
        for e in &self.entries {
            let p = norm(&e.prefix);
            let hit = seg.starts_with(&p) || full.starts_with(&p);
            if hit {
                let len = p.len();
                if best.is_none_or(|(l, _)| len >= l) {
                    best = Some((len, e.price));
                }
            }
        }
        best.map(|(_, v)| v)
    }
}
