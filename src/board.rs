//! `costcrew leaderboard` — arena-style model ranking, priced.
//!
//! Scores: bundled snapshot of LMArena's Coding category (blind pairwise votes,
//! Bradley-Terry/Elo). Prices: merged from the built-in table + the cached
//! OpenRouter list (see `costcrew fetch-prices`).
//!
//! The value column is tuned for how coding AGENTS actually burn tokens:
//! agent_blend = 0.2·input + 0.1·output + 0.7·cache_read (per 1M tokens),
//! from real OpenCode session mixes (cache dwarfs everything in agentic loops).
//! value = (elo − 1400) / agent_blend  → points per dollar.

use crate::price::PriceDB;
use crate::report::{BOLD, DIM, GREEN, RED, RESET, YELLOW};

pub struct BoardEntry {
    /// prefix that must resolve through the price DB
    pub prefix: &'static str,
    pub display: &'static str,
    pub elo: f64,
    pub votes: u64,
}

/// Snapshot: LMArena Coding category, Nov 2026 (~2.0M votes). Best-effort row
/// per model (highest reasoning effort). Elo only — margins are ±4–19 pts.
#[rustfmt::skip]
pub static BOARD: &[BoardEntry] = &[
    BoardEntry { prefix: "gemini-4-carbon",     display: "Gemini 4 Carbon   · Google",      elo: 1564.0, votes: 1069 },
    BoardEntry { prefix: "gemini-4-argon",      display: "Gemini 4 Argon    · Google",      elo: 1553.0, votes: 4440 },
    BoardEntry { prefix: "claude-fable-5-1",    display: "Claude Fable 5.1  · Anthropic",   elo: 1534.0, votes: 3870 },
    BoardEntry { prefix: "claude-fable-5",      display: "Claude Fable 5    · Anthropic",   elo: 1551.0, votes: 11686 },
    BoardEntry { prefix: "claude-opus-4-7",     display: "Claude Opus 4.7   · Anthropic",   elo: 1546.0, votes: 18359 },
    BoardEntry { prefix: "claude-opus-4-6",     display: "Claude Opus 4.6   · Anthropic",   elo: 1547.0, votes: 24575 },
    BoardEntry { prefix: "claude-opus-4-5",     display: "Claude Opus 4.5   · Anthropic",   elo: 1530.0, votes: 7752 },
    BoardEntry { prefix: "claude-opus-5-5",     display: "Claude Opus 5.5   · Anthropic",   elo: 1546.0, votes: 3257 },
    BoardEntry { prefix: "claude-opus-5",       display: "Claude Opus 5     · Anthropic",   elo: 1532.0, votes: 18838 },
    BoardEntry { prefix: "claude-sonnet-5-5",   display: "Claude Sonnet 5.5 · Anthropic",   elo: 1517.0, votes: 3109 },
    BoardEntry { prefix: "claude-sonnet-5",     display: "Claude Sonnet 5   · Anthropic",   elo: 1520.0, votes: 14082 },
    BoardEntry { prefix: "claude-sonnet-4-6",   display: "Claude Sonnet 4.6 · Anthropic",   elo: 1528.0, votes: 19474 },
    BoardEntry { prefix: "claude-haiku-5-5",    display: "Claude Haiku 5.5  · Anthropic",   elo: 1455.0, votes: 3003 },
    BoardEntry { prefix: "claude-haiku-4-5",    display: "Claude Haiku 4.5  · Anthropic",   elo: 1453.0, votes: 39588 },
    BoardEntry { prefix: "gpt-6-astra",         display: "GPT-6 Astra       · OpenAI",      elo: 1548.0, votes: 4096 },
    BoardEntry { prefix: "gpt-6-1-sol",         display: "GPT-6.1 Sol       · OpenAI",      elo: 1541.0, votes: 2668 },
    BoardEntry { prefix: "gpt-6-sol",           display: "GPT-6 Sol         · OpenAI",      elo: 1519.0, votes: 4530 },
    BoardEntry { prefix: "gpt-6-luna",          display: "GPT-6 Luna        · OpenAI",      elo: 1501.0, votes: 24306 },
    BoardEntry { prefix: "gpt-5-6-sol",         display: "GPT-5.6 Sol       · OpenAI",      elo: 1533.0, votes: 11621 },
    BoardEntry { prefix: "gpt-5-5",             display: "GPT-5.5           · OpenAI",      elo: 1510.0, votes: 19751 },
    BoardEntry { prefix: "gpt-5-4",             display: "GPT-5.4           · OpenAI",      elo: 1521.0, votes: 17419 },
    BoardEntry { prefix: "gpt-5-2",             display: "GPT-5.2           · OpenAI",      elo: 1442.0, votes: 12151 },
    BoardEntry { prefix: "muse-wonder",         display: "Muse Wonder       · Meta",        elo: 1546.0, votes: 2780 },
    BoardEntry { prefix: "muse-spark-1-3",      display: "Muse Spark 1.3    · Meta",        elo: 1532.0, votes: 6575 },
    BoardEntry { prefix: "kimi-k3",             display: "Kimi K3           · Moonshot",    elo: 1537.0, votes: 10072 },
    BoardEntry { prefix: "kimi-k2-6",           display: "Kimi K2.6         · Moonshot",    elo: 1516.0, votes: 10982 },
    BoardEntry { prefix: "glm-5-3",             display: "GLM-5.3           · Z.ai",        elo: 1522.0, votes: 7164 },
    BoardEntry { prefix: "glm-5-3-flash",       display: "GLM-5.3-Flash     · Z.ai",        elo: 1527.0, votes: 12519 },
    BoardEntry { prefix: "glm-5-2",             display: "GLM-5.2           · Z.ai",        elo: 1515.0, votes: 14375 },
    BoardEntry { prefix: "glm-4-6",             display: "GLM 4.6           · Z.ai",        elo: 1449.0, votes: 7577 },
    BoardEntry { prefix: "deepseek-v4-1-flash", display: "DeepSeek V4.1 Flash · DeepSeek",  elo: 1525.0, votes: 3719 },
    BoardEntry { prefix: "deepseek-v4-pro",     display: "DeepSeek V4 Pro   · DeepSeek",    elo: 1503.0, votes: 2756 },
    BoardEntry { prefix: "qwen3-8-max",         display: "Qwen3.8-Max       · Alibaba",     elo: 1524.0, votes: 9596 },
    BoardEntry { prefix: "qwen3-8-27b",         display: "Qwen3.8-27B       · Alibaba",     elo: 1478.0, votes: 8592 },
    BoardEntry { prefix: "grok-4-6",            display: "Grok 4.6          · xAI",         elo: 1508.0, votes: 8571 },
    BoardEntry { prefix: "mimo-v2-6-pro",       display: "MiMo-V2.6-Pro     · Xiaomi",      elo: 1538.0, votes: 1095 },
    BoardEntry { prefix: "mimo-v2-6-flash",     display: "MiMo-V2.6-Flash   · Xiaomi",      elo: 1511.0, votes: 5033 },
    BoardEntry { prefix: "gemini-3-8-flash",    display: "Gemini 3.8 Flash  · Google",      elo: 1530.0, votes: 15174 },
    BoardEntry { prefix: "gemini-3-7-flash",    display: "Gemini 3.7 Flash  · Google",      elo: 1520.0, votes: 8973 },
    BoardEntry { prefix: "gemini-3-6-flash",    display: "Gemini 3.6 Flash  · Google",      elo: 1518.0, votes: 10179 },
    BoardEntry { prefix: "gemini-2-5-pro",      display: "Gemini 2.5 Pro    · Google",      elo: 1452.0, votes: 26810 },
];

/// a representative agentic token blend (fractions of billable tokens)
const BLEND_IN: f64 = 0.2;
const BLEND_OUT: f64 = 0.1;
const BLEND_CACHE: f64 = 0.7;
const ELO_BASELINE: f64 = 1400.0;

fn agent_blend(p: crate::price::Pricing) -> f64 {
    BLEND_IN * p.input + BLEND_OUT * p.output + BLEND_CACHE * p.cache_read
}

#[derive(Clone, Copy, PartialEq)]
pub enum Sort {
    Elo,
    Value,
    Blend,
}

pub fn parse_sort(s: &str) -> Result<Sort, String> {
    match s {
        "elo" | "score" => Ok(Sort::Elo),
        "value" => Ok(Sort::Value),
        "price" | "cost" | "blend" => Ok(Sort::Blend),
        _ => Err("--sort is elo | value | price".into()),
    }
}

pub fn print_board(db: &PriceDB, sort: Sort, top: usize, json: bool) {
    let mut rows: Vec<(&BoardEntry, Option<f64>, Option<f64>)> = BOARD
        .iter()
        .map(|b| {
            let blend = db.lookup(b.prefix).map(agent_blend);
            let value = blend.map(|blend| (b.elo - ELO_BASELINE) / blend);
            (b, blend, value)
        })
        .collect();

    match sort {
        Sort::Elo => rows.sort_by(|a, b| {
            b.0.elo
                .partial_cmp(&a.0.elo)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Sort::Value => rows.sort_by(|a, b| {
            b.2.unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&a.2.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Sort::Blend => rows.sort_by(|a, b| {
            a.1.unwrap_or(f64::INFINITY)
                .partial_cmp(&b.1.unwrap_or(f64::INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    if json {
        use serde_json::json;
        let arr: Vec<_> = rows
            .iter()
            .take(top)
            .map(|(b, blend, value)| {
                json!({
                    "model": b.display.trim(),
                    "prefix": b.prefix,
                    "arena_elo": b.elo,
                    "votes": b.votes,
                    "agent_blend_usd_per_1m": blend.map(|x| (x * 100.0).round() / 100.0),
                    "value_elo_per_dollar": value.map(|x| (x * 10.0).round() / 10.0),
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "source": "LMArena Coding snapshot Nov 2026",
                "blend": {"input": BLEND_IN, "output": BLEND_OUT, "cache_read": BLEND_CACHE},
                "rows": arr,
            })
        );
        return;
    }

    println!();
    println!("  {BOLD}costcrew leaderboard{RESET} {DIM}· LMArena Coding snapshot Nov 2026 · ~2.0M blind votes{RESET}");
    println!();
    println!(
        "  {:>3} {:<30} {:>5} {:>7} {:>8} {:>8} {:>9} {:>10}",
        "#", "model", "elo", "votes", "in $", "out $", "agent$", "elo/$"
    );
    println!(
        "  {}",
        "-".repeat(3 + 1 + 30 + 1 + 5 + 1 + 7 + 1 + 8 + 1 + 8 + 1 + 9 + 1 + 10)
    );

    for (i, (b, blend, value)) in rows.iter().take(top).enumerate() {
        let blend = *blend;
        let value = *value;
        let (bi, bo) = match blend {
            Some(_) => {
                let p = db.lookup(b.prefix).expect("blend implies price");
                (p.input, p.output)
            }
            None => (f64::NAN, f64::NAN),
        };
        let price_str = |v: f64| {
            if blend.is_some() {
                format!("{:>7.2}", v)
            } else {
                "      ?".to_string()
            }
        };
        let blend_str = match blend {
            Some(blend) => format!("{:.2}", blend),
            None => "?".into(),
        };
        let value_str = match value {
            Some(v) if v.is_finite() => format!("{:.1}", v),
            _ => "-".into(),
        };
        let color = match value {
            Some(v) if blend.is_some_and(|blend| blend > 0.0) && v >= 25.0 => GREEN,
            Some(v) if blend.is_some_and(|blend| blend > 0.0) && v >= 8.0 => YELLOW,
            _ if value.is_some() => RED,
            _ => DIM,
        };
        println!(
            "  {:>3} {:<30} {:>5.0} {:>7} {} {} {:>9} {color}{:>10}{RESET}",
            i + 1,
            b.display,
            b.elo,
            format_votes(b.votes),
            price_str(bi),
            price_str(bo),
            blend_str,
            value_str,
            color = color,
        );
    }

    if rows.iter().all(|(_, _, _)| true) && rows.iter().any(|(_, blend, _)| blend.is_none()) {
        println!();
        println!("  {DIM}? = model not in your price sources yet. `costcrew fetch-prices` may fix it.{RESET}");
    }
    println!();
    println!(
        "  {DIM}agent$ = typical agentic blend: 20% in + 10% out + 70% cache-read (per 1M).{RESET}"
    );
    println!("  {DIM}elo/$  = (elo − 1400) ÷ agent$ — value points per dollar{RESET}");
    println!("  {DIM}snapshot: arena.ai / lmarena.ai — bump `src/board.rs` with a PR{RESET}");
    println!();
}

pub fn format_votes(v: u64) -> String {
    if v >= 1000 {
        format!("{:.1}k", v as f64 / 1000.0)
    } else {
        format!("{v}")
    }
}
