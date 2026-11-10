//! Live pricing + spend from OpenRouter.
//!
//! Two uses:
//!   1. `--fetch-prices` pulls every OpenRouter model's price (public endpoint,
//!      no key) and caches it locally — covers "every model, automatically".
//!   2. `costcrew openrouter` reads your OPENROUTER_API_KEY and reports what
//!      you actually spent, per model, from the activity endpoint.

use crate::price::{PriceDB, Pricing};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const ACTIVITY_URL: &str = "https://openrouter.ai/api/v1/activity?limit=200";
const CACHE_VERSION: u64 = 1;

pub fn cache_dir() -> PathBuf {
    if let Some(lad) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(lad).join("costcrew");
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/costcrew")
}

fn num(s: &Value) -> f64 {
    match s {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Public models endpoint -> per-1M USD prices, cached on disk.
pub fn force_refresh() -> Result<usize, String> {
    let resp = ureq::get(MODELS_URL)
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|e| format!("fetch failed: {e}"))?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let mut models: Vec<(String, [f64; 4])> = Vec::new();
    if let Some(list) = v.get("data").and_then(Value::as_array) {
        for m in list {
            let id = match m.get("id").and_then(Value::as_str) {
                Some(id) => id.to_string(),
                None => continue,
            };
            // pricing is per-token strings; scale to per-1M
            let p = m.get("pricing").cloned().unwrap_or(Value::Null);
            let scale = 1_000_000.0;
            let price = [
                num(&p.get("prompt").cloned().unwrap_or(Value::Null)) * scale,
                num(&p.get("completion").cloned().unwrap_or(Value::Null)) * scale,
                num(&p.get("input_cache_read").cloned().unwrap_or(Value::Null)) * scale,
                num(&p.get("input_cache_write").cloned().unwrap_or(Value::Null)) * scale,
            ];
            models.push((id, price));
        }
    }
    if models.is_empty() {
        return Err("openrouter returned no models".into());
    }
    let doc = serde_json::json!({
        "version": CACHE_VERSION,
        "fetched_at_ms": now_ms(),
        "models": models.iter().map(|(id, p)| serde_json::json!({
            "id": id, "prompt": p[0], "completion": p[1], "cache_read": p[2], "cache_write": p[3],
        })).collect::<Vec<_>>(),
    });
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("openrouter-prices.json"), doc.to_string())
        .map_err(|e| e.to_string())?;
    Ok(models.len())
}

pub fn cached_into_db(db: &mut PriceDB) -> usize {
    let path = cache_dir().join("openrouter-prices.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return 0;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return 0;
    };
    if v.get("version").and_then(Value::as_u64) != Some(CACHE_VERSION) {
        return 0;
    }
    let mut n = 0usize;
    if let Some(list) = v.get("models").and_then(Value::as_array) {
        for m in list {
            let id = match m.get("id").and_then(Value::as_str) {
                Some(s) => s.to_string(),
                None => continue,
            };
            db.push(
                &id,
                Pricing {
                    input: num(&m["prompt"]),
                    output: num(&m["completion"]),
                    cache_read: num(&m["cache_read"]),
                    cache_write: num(&m["cache_write"]),
                    per_hour: 0.0,
                },
            );
            n += 1;
        }
    }
    db.fetched_count = n;
    n
}

/// Actual spend per model for an OpenRouter API key.
pub fn activity(key: &str) -> Result<Vec<(String, u64, f64)>, String> {
    let resp = ureq::get(ACTIVITY_URL)
        .timeout(Duration::from_secs(30))
        .set("Authorization", &format!("Bearer {key}"))
        .call()
        .map_err(|e| format!("openrouter request failed: {e}"))?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(msg) = v.get("error") {
        return Err(format!("openrouter error: {msg}"));
    }

    let mut by_model: Vec<(String, u64, f64)> = Vec::new();
    if let Some(list) = v.get("data").and_then(Value::as_array) {
        for ev in list {
            let model = match ev.get("model").and_then(Value::as_str) {
                Some(m) => m.to_string(),
                None => continue,
            };
            let cost = num(ev.get("cost").unwrap_or(&Value::Null));
            if let Some(row) = by_model.iter_mut().find(|(m, _, _)| *m == model) {
                row.1 += 1;
                row.2 += cost;
            } else {
                by_model.push((model, 1, cost));
            }
        }
    }
    by_model.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    Ok(by_model)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
