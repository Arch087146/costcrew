//! `costcrew advise` — should you have used a cheaper model?
//!
//! Honest heuristic mode: recomputes your ACTUAL token mix under cheaper
//! alternatives that publish benchmark tiers at/above the model you used.
//! Quality scores are approximate tiers from public evals — the goal is to
//! surface waste, not to write your model-selection policy.

use crate::price::PriceDB;
use crate::report::*;
use crate::scan::Row;

/// Approximate coding-benchmark tier per model family (5 = frontier).
/// Rounded, contested, and deliberately conservative: advise only suggests
/// swapping when the *candidate's tier is >= the model you used*.
const TIERS: &[(&str, u8)] = &[
    ("claude-opus", 5),
    ("gpt-5", 5),
    ("grok-4", 5),
    ("gemini-3", 5),
    ("kimi-k2", 4),
    ("claude-sonnet", 4),
    ("glm-5", 4),
    ("deepseek-reasoner", 4),
    ("glm-4.6", 3),
    ("deepseek-chat", 3),
    ("qwen", 3),
    ("llama-4", 3),
    ("mistral-large", 3),
    ("gemini-2", 3),
    ("gpt-4.1", 3),
    ("gpt-4o", 3),
    ("o3", 4),
    ("claude-haiku", 3),
    ("mistral-small", 2),
    ("devstral", 2),
    ("codestral", 2),
    ("mistral", 2),
    ("minimax", 2),
    ("command", 2),
    ("phi", 2),
];

/// Candidate pool for downgrades — cheapest-first, all tiers.
const CANDIDATES: &[&str] = &[
    "claude-haiku-4",
    "glm-4.6",
    "deepseek-chat",
    "qwen3",
    "mistral-small",
    "devstral",
    "kimi-k2",
    "glm-5",
    "deepseek-reasoner",
    "claude-sonnet-4",
    "gemini-3",
    "gpt-5",
    "claude-opus-4",
];

fn tier_of(model: &str) -> u8 {
    let m = model.to_lowercase();
    for (k, t) in TIERS {
        if m.starts_with(k) {
            return *t;
        }
    }
    0
}

/// What this row's token mix would cost on an alternative model.
fn est_cost(row: &Row, alt: &str, db: &PriceDB) -> Option<f64> {
    let p = db.lookup(alt)?;
    Some(
        (row.input_tokens as f64 / 1e6) * p.input
            + (row.output_tokens as f64 / 1e6) * p.output
            + (row.cache_read_tokens as f64 / 1e6) * p.cache_read
            + (row.cache_write_tokens as f64 / 1e6) * p.cache_write
            + p.per_hour * (row.duration_secs / 3600.0),
    )
}

pub fn print_advise(rows: &[Row], db: &PriceDB, since_days: u32) {
    println!();
    println!(
        "  {BOLD}costcrew advise{RESET} {DIM}· cheaper-model verdicts · last {since_days}d{RESET}"
    );
    println!();

    if !rows.iter().any(|r| r.known_price && r.requests > 0) {
        println!("  {DIM}nothing priced to advise on — run `costcrew` first.{RESET}");
        return;
    }

    let mut suggested_ok = 0usize;
    for r in rows.iter().filter(|r| r.known_price && r.cost > 0.01) {
        let my_tier = tier_of(&r.model);
        if my_tier == 0 {
            // don't pretend to know about unknown-model quality
            continue;
        }
        let mut alts: Vec<(f64, &str, u8)> = CANDIDATES
            .iter()
            .filter(|c| tier_of(c) >= my_tier)
            .filter_map(|c| est_cost(r, c, db).map(|cost| (cost, *c, tier_of(c))))
            .filter(|(cost, _, _)| *cost < r.cost * 0.9)
            .collect();
        alts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((alt_cost, alt_name, alt_tier)) = alts.first().copied() {
            let save = r.cost - alt_cost;
            let pct = (1.0 - alt_cost / r.cost).max(0.0) * 100.0;
            let tier_note = if alt_tier == my_tier {
                "same tier"
            } else {
                "higher tier"
            };
            println!(
                "  {YELLOW}{:>30}{RESET}  ->  {GREEN}{:<22}{RESET} {DIM}({} · probable save ${:.2} ({:.0}%)){RESET}",
                truncate(&r.model, 30),
                alt_name,
                tier_note,
                save,
                pct
            );
            println!(
                "  {:>30}  {DIM}   same work would have cost ${:.2} instead of ${:.2}{RESET}",
                "", alt_cost, r.cost
            );
            suggested_ok += 1;
        }
    }

    if suggested_ok == 0 {
        println!("  {GREEN}✓ no waste pattern detected in the last {since_days}d — your tiers match your spend.{RESET}");
    } else {
        println!();
        println!(
            "  {DIM}heuristic: tiers from public coding evals, costs = your exact token mix{RESET}"
        );
        println!(
            "  {DIM}at those prices. quality isn't guaranteed — run one task as an A/B and{RESET}"
        );
        println!("  {DIM}compare before rewiring your whole workflow.{RESET}");
    }
    println!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
