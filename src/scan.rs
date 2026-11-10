//! Scans local logs/transcripts of AI coding agents for token usage.

use crate::price;
use serde_json::Value;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Default)]
pub struct UsageEvent {
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    /// wall-clock of the request, when the log records it (opencode does)
    pub duration_ms: u64,
    /// when the agent itself reports a cost (Kilo/Cline/Roo do), USD
    pub direct_cost: Option<f64>,
    #[allow(dead_code)]
    pub source: PathBuf,
}

#[derive(Debug, Default)]
pub struct Row {
    pub provider: String,
    pub model: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_tokens: u64,
    pub duration_secs: f64,
    pub direct_cost_sum: f64,
    pub cost: f64,
    pub known_price: bool,
}

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// (directory, provider) pairs for every tool we know how to read.
fn scan_targets() -> Vec<(PathBuf, &'static str)> {
    let h = home();
    let mut targets = vec![
        // Claude Code: JSONL transcripts per project
        (h.join(".claude"), "claude"),
    ];
    // OpenCode: legacy JSON storage + whatever lives in its data dir
    let oc = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .map(|p| p.join("opencode"));
    targets.push((
        oc.unwrap_or_else(|| h.join(".local/share/opencode")),
        "opencode",
    ));
    if let Some(lad) = std::env::var_os("LOCALAPPDATA") {
        targets.push((PathBuf::from(lad).join("opencode"), "opencode"));
    }
    // Cline-family VS Code agents (Kilo Code, Cline, Roo Code) log tasks here.
    // Prefer APPDATA (where VS Code globalStorage actually lives); fall back to
    // LOCALAPPDATA's parent so the scanner still runs on unusual setups.
    let mut gs: Option<PathBuf> = std::env::var_os("APPDATA").map(|a| {
        PathBuf::from(a)
            .join("Code")
            .join("User")
            .join("globalStorage")
    });
    if gs.is_none() {
        gs = std::env::var_os("LOCALAPPDATA").map(|l| {
            PathBuf::from(&l)
                .parent()
                .map(|p| {
                    p.join("Roaming")
                        .join("Code")
                        .join("User")
                        .join("globalStorage")
                })
                .unwrap_or_else(|| PathBuf::from(&l))
        });
    }
    if let Some(gs) = gs {
        targets.push((gs, "cline-family"));
    }
    // Codex CLI session logs
    targets.push((h.join(".codex"), "codex"));
    targets
}

pub fn collect(since_days: u32) -> Vec<UsageEvent> {
    let max_age = Duration::from_secs(since_days as u64 * 86_400);
    let since_ms = (now_ms() - max_age.as_millis() as i64).max(0);
    let mut events = Vec::new();

    for (dir, provider) in scan_targets() {
        if !dir.is_dir() {
            continue;
        }
        // v2 agents keep usage in SQLite (opencode.db); read that first.
        let db = dir.join("opencode.db");
        if db.is_file() {
            scan_opencode_db(&db, provider, since_ms, &mut events);
        }
        walk(&dir, provider, 0, &mut |path, mtime| {
            if let Ok(age) = SystemTime::now().duration_since(mtime) {
                if age > max_age {
                    return;
                }
            }
            let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
            if ext.eq_ignore_ascii_case("jsonl") {
                parse_jsonl(path, provider, &mut events);
            } else if ext.eq_ignore_ascii_case("json") {
                parse_single_json(path, provider, &mut events);
            }
        });
    }

    // Aider: cost lines in its chat history markdown
    scan_aider_history(since_days, &mut events);
    events
}

/// Aider appends "> Cost: $a ask, $b send, $c total." lines to history .md files.
/// We trust its own numbers (one event per history line).
fn scan_aider_history(since_days: u32, events: &mut Vec<UsageEvent>) {
    let h = home();
    for name in [".aider.chat.history.md", ".aider.history.md"] {
        let path = h.join(name);
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else { continue };
        if let Ok(age) = SystemTime::now().duration_since(mtime) {
            if age > Duration::from_secs(since_days as u64 * 86_400) {
                continue;
            }
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if !line.contains("Cost:") {
                continue;
            }
            // take the largest $ figure on the line (usually the total)
            let mut biggest = 0.0f64;
            let mut start = 0;
            while let Some(i) = line[start..].find('$') {
                let rest = &line[start + i + 1..];
                let num: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(v) = num.parse::<f64>() {
                    biggest = biggest.max(v);
                }
                start += i + 1 + num.len();
                if start >= line.len() {
                    break;
                }
            }
            if biggest <= 0.0 {
                continue;
            }
            events.push(UsageEvent {
                provider: "aider".into(),
                model: "aider-reported".into(),
                duration_ms: 0,
                direct_cost: Some(biggest),
                source: path.clone(),
                ..Default::default()
            });
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// OpenCode v2 keeps one `data` JSON blob per message in opencode.db.
fn scan_opencode_db(path: &Path, provider: &str, since_ms: i64, events: &mut Vec<UsageEvent>) {
    use rusqlite::{Connection, OpenFlags};

    let Ok(conn) = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return;
    };
    let Ok(mut stmt) = conn.prepare("SELECT time_created, data FROM message") else {
        return;
    };
    let mut rows = match stmt.query([]) {
        Ok(r) => r,
        Err(_) => return,
    };
    while let Ok(Some(row)) = rows.next() {
        let created: i64 = row.get(0).unwrap_or(0);
        if created > 0 && created < since_ms {
            continue;
        }
        let Ok(data) = row.get::<_, String>(1) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&data) else {
            continue;
        };
        if let Some(ev) = extract_event(&v, provider, path) {
            events.push(ev);
        }
    }
}

fn walk(dir: &Path, provider: &str, depth: usize, cb: &mut impl FnMut(&Path, SystemTime)) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        // skip noisy dirs that never hold transcripts
        if matches!(
            name.as_ref(),
            "node_modules" | "target" | "dist" | "tmp" | "cache"
        ) {
            continue;
        }
        if p.is_dir() {
            walk(&p, provider, depth + 1, cb);
        } else if let Ok(meta) = e.metadata() {
            if let Ok(mtime) = meta.modified() {
                cb(&p, mtime);
            }
        }
    }
}

fn parse_jsonl(path: &Path, provider: &str, events: &mut Vec<UsageEvent>) {
    let Ok(file) = File::open(path) else { return };
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { continue };
        if !(line.contains("usage") || line.contains("tokens")) {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(ev) = extract_event(&v, provider, path) {
            events.push(ev);
        }
    }
}

fn parse_single_json(path: &Path, provider: &str, events: &mut Vec<UsageEvent>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    if text.len() > 8_000_000 {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    match &v {
        // Cline-family ui_messages/task files are arrays
        Value::Array(items) => {
            for item in items {
                if let Some(ev) = extract_event(item, provider, path) {
                    events.push(ev);
                }
                dig_for_cline_usage(item, provider, path, events, 0);
            }
        }
        Value::Object(_) => {
            if let Some(ev) = extract_event(&v, provider, path) {
                events.push(ev);
            }
            dig_for_cline_usage(&v, provider, path, events, 0);
        }
        _ => {}
    }
}

/// Kilo Code / Cline / Roo Code wrap per-request cost JSON inside string
/// fields ("text") inside their task files — unwrap those one level.
fn dig_for_cline_usage(
    v: &Value,
    provider: &str,
    path: &Path,
    events: &mut Vec<UsageEvent>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    match v {
        Value::Array(items) => {
            for i in items {
                dig_for_cline_usage(i, provider, path, events, depth + 1);
            }
        }
        Value::Object(map) => {
            for val in map.values() {
                dig_for_cline_usage(val, provider, path, events, depth + 1);
            }
            if let Some(Value::String(s)) = map.get("text") {
                if s.contains("tokensIn") || s.contains("cacheReads") || s.contains("\"cost\"") {
                    if let Ok(inner) = serde_json::from_str::<Value>(s) {
                        if let Some(ev) = extract_event(&inner, provider, path) {
                            events.push(ev);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn num(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|k| {
            obj.get(*k)
                .and_then(Value::as_u64)
                .or_else(|| obj.get(*k).and_then(Value::as_f64).map(|f| f as u64))
        })
        .unwrap_or(0)
}

fn extract_event(v: &Value, provider: &str, source: &Path) -> Option<UsageEvent> {
    // usage can live at: v.usage | v.tokens | v.message.usage
    let usage = v
        .get("usage")
        .or_else(|| v.get("tokens"))
        .or_else(|| v.get("message").and_then(|m| m.get("usage")))
        .filter(|u| u.is_object())?;

    let model = ["model", "modelID", "model_id"]
        .iter()
        .find_map(|k| v.get(k).and_then(Value::as_str))
        .or_else(|| {
            v.get("message").and_then(|m| {
                m.get("model")
                    .and_then(Value::as_str)
                    .or_else(|| m.get("modelID").and_then(Value::as_str))
            })
        })?
        .to_string();

    if model.is_empty() {
        return None;
    }

    // prefer the provider id stored next to the model (e.g. openai, anthropic)
    let provider_name = ["providerID", "provider_id", "provider", "apiProvider"]
        .iter()
        .find_map(|k| v.get(k).and_then(Value::as_str))
        .unwrap_or(provider);

    let um = usage.as_object()?;

    let input = num(um, &["input_tokens", "inputTokens", "tokensIn", "input"]);
    // reasoning tokens are billed as output on most APIs — fold them in
    let output = num(
        um,
        &["output_tokens", "outputTokens", "tokensOut", "output"],
    ) + num(um, &["reasoning_tokens", "reasoningTokens", "reasoning"]);
    let mut cache_read = num(
        um,
        &[
            "cache_read_input_tokens",
            "cacheReadTokens",
            "cacheReads",
            "cache_read",
            "cacheRead",
        ],
    );
    let mut cache_write = num(
        um,
        &[
            "cache_creation_input_tokens",
            "cacheWriteTokens",
            "cacheWrites",
            "cache_creation",
            "cacheWrite",
        ],
    );

    // nested cache object: "cache": { "read": n, "write": n }
    if let Some(c) = um.get("cache").and_then(Value::as_object) {
        cache_read = cache_read.max(num(c, &["read", "cacheRead"]));
        cache_write = cache_write.max(num(c, &["write", "cacheWrite"]));
    }

    // Cline-family: agent-reported USD cost of the request
    let direct_cost = v
        .get("cost")
        .or_else(|| um.get("cost"))
        .and_then(fnum)
        .filter(|c| *c > 0.0);

    // OpenCode records request wall-clock; needed for per-hour pricing
    let duration_ms = v
        .get("time")
        .and_then(|t| {
            t.get("completed")
                .and_then(Value::as_i64)
                .zip(t.get("created").and_then(Value::as_i64))
        })
        .map(|(a, b)| (a - b).max(0) as u64)
        .unwrap_or(0);

    if input == 0 && output == 0 && cache_read == 0 && cache_write == 0 && direct_cost.is_none() {
        return None;
    }

    Some(UsageEvent {
        provider: provider_name.to_string(),
        model,
        input_tokens: input,
        output_tokens: output,
        cache_read,
        cache_write,
        duration_ms,
        direct_cost,
        source: source.to_path_buf(),
    })
}

/// numbers that arrive as strings (Cline serializes tons as strings)
fn fnum(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn aggregate(events: &[UsageEvent], db: &price::PriceDB) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let bump = |r: &mut Row, e: &UsageEvent| {
        r.requests += 1;
        r.input_tokens += e.input_tokens;
        r.output_tokens += e.output_tokens;
        r.cache_read_tokens += e.cache_read;
        r.cache_write_tokens += e.cache_write;
        r.cache_tokens += e.cache_read + e.cache_write;
        r.duration_secs += e.duration_ms as f64 / 1000.0;
        if let Some(c) = e.direct_cost {
            r.direct_cost_sum += c;
        }
    };
    for e in events {
        if let Some(r) = rows
            .iter_mut()
            .find(|r| r.provider == e.provider && r.model == e.model)
        {
            bump(r, e);
        } else {
            rows.push(Row {
                provider: e.provider.clone(),
                model: e.model.clone(),
                requests: 1,
                input_tokens: e.input_tokens,
                output_tokens: e.output_tokens,
                cache_read_tokens: e.cache_read,
                cache_write_tokens: e.cache_write,
                cache_tokens: e.cache_read + e.cache_write,
                duration_secs: e.duration_ms as f64 / 1000.0,
                direct_cost_sum: e.direct_cost.unwrap_or(0.0),
                cost: 0.0,
                known_price: false,
            });
        }
    }
    for r in &mut rows {
        // 1) agent-reported costs win (Kilo/Cline/Roo say the price themselves)
        if r.direct_cost_sum > 0.0 {
            r.known_price = true;
            r.cost = r.direct_cost_sum;
            continue;
        }
        // 2) token + wall-clock pricing from the database
        match db.lookup(&r.model) {
            Some(p) => {
                r.known_price = true;
                r.cost = (r.input_tokens as f64 / 1e6) * p.input
                    + (r.output_tokens as f64 / 1e6) * p.output
                    + (r.cache_read_tokens as f64 / 1e6) * p.cache_read
                    + (r.cache_write_tokens as f64 / 1e6) * p.cache_write
                    + p.per_hour * (r.duration_secs / 3600.0);
            }
            None => {
                r.known_price = false;
                r.cost = 0.0;
            }
        }
    }
    rows.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(model: &str, i: u64, o: u64) -> UsageEvent {
        UsageEvent {
            provider: "test".into(),
            model: model.into(),
            input_tokens: i,
            output_tokens: o,
            cache_read: 0,
            cache_write: 0,
            duration_ms: 0,
            direct_cost: None,
            source: "t".into(),
        }
    }

    #[test]
    fn aggregate_sums_and_sorts_by_cost() {
        let db = price::PriceDB::builtin();
        let rows = aggregate(
            &[ev("gpt-5", 1_000_000, 0), ev("gpt-5", 500_000, 100_000)],
            &db,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].requests, 2);
        assert_eq!(rows[0].input_tokens, 1_500_000);
        assert_eq!(rows[0].output_tokens, 100_000);
    }

    #[test]
    fn unknown_models_are_flagged_not_billed() {
        let db = price::PriceDB::builtin();
        let rows = aggregate(&[ev("totally-made-up-model", 1_000_000, 1_000)], &db);
        assert!(!rows[0].known_price);
        assert_eq!(rows[0].cost, 0.0);
    }

    #[test]
    fn agent_reported_cost_wins_over_estimate() {
        let db = price::PriceDB::builtin();
        let mut e = ev("gpt-5", 1_000_000, 0);
        e.direct_cost = Some(0.42);
        let rows = aggregate(&[e], &db);
        assert!(rows[0].known_price);
        assert!((rows[0].cost - 0.42).abs() < 1e-9);
    }

    #[test]
    fn pricing_prefix_match_hits_longest() {
        let db = price::PriceDB::builtin();
        let p = db.lookup("claude-opus-4-5-20990101").unwrap();
        assert_eq!(p.input, 15.0);
        let p = db.lookup("glm-4-air").unwrap();
        assert_eq!(p.input, 0.6);
        // OpenRouter-style slug resolves to the same entry
        let p = db.lookup("anthropic/claude-opus-4-1").unwrap();
        assert_eq!(p.input, 15.0);
    }

    #[test]
    fn dot_slash_underscore_separators_match() {
        let db = price::PriceDB::builtin();
        // OpenRouter slugs use dots; some agents use underscores
        assert!(db.lookup("anthropic/claude.opus.4.5").is_some());
        assert!(db.lookup("openai/gpt_5_2").is_some());
        assert!(db.lookup("zai/glm-5.3").is_some());
    }
}
