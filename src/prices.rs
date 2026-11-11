//! `costcrew prices` — browse the bundled price database.
//!
//! The database ships with the binary: a curated vendor table plus the full
//! embedded OpenRouter catalog (data/prices.json), optionally enriched by the
//! on-disk cache (`fetch-prices`) and custom overrides (`--prices`). No
//! network, no log scanning — it's just a database you can read.

use crate::price::PriceDB;
use crate::report::{BOLD, CYAN, DIM, GREEN, RESET, YELLOW};

pub fn print_prices(db: &PriceDB, query: Option<&str>, sort: &str, top: usize, json: bool) {
    // filter + dedupe: the same model can appear in curated + embedded +
    // cache + custom; the LATEST entry is authoritative (matches lookup order)
    let mut latest: std::collections::HashMap<String, &crate::price::PriceEntry> =
        std::collections::HashMap::new();
    for e in db.entries() {
        if let Some(q) = query {
            if !e.prefix.to_lowercase().contains(&q.to_lowercase()) {
                continue;
            }
        }
        latest.insert(e.prefix.clone(), e);
    }
    let mut rows: Vec<&crate::price::PriceEntry> = latest.into_values().collect();

    match sort {
        "price" | "cost" => rows.sort_by(|a, b| {
            let ka = a.price.input + a.price.output;
            let kb = b.price.input + b.price.output;
            kb.partial_cmp(&ka).unwrap_or(std::cmp::Ordering::Equal) // priciest first
        }),
        "cheap" => rows.sort_by(|a, b| {
            let ka = a.price.input + a.price.output;
            let kb = b.price.input + b.price.output;
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal) // cheapest first
        }),
        _ => rows.sort_by(|a, b| a.prefix.cmp(&b.prefix)), // name
    }

    if json {
        use serde_json::json;
        let arr: Vec<_> = rows
            .iter()
            .take(top.max(1))
            .map(|e| {
                json!({
                    "model": e.prefix,
                    "input_per_1m": e.price.input,
                    "output_per_1m": e.price.output,
                    "cache_read_per_1m": e.price.cache_read,
                    "cache_write_per_1m": e.price.cache_write,
                    "per_hour": if e.price.per_hour > 0.0 { serde_json::Value::from(e.price.per_hour) } else { serde_json::Value::Null },
                    "source": e.src,
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "total_models": db.entries().len(),
                "matches": rows.len(),
                "rows": arr,
            })
        );
        return;
    }

    println!();
    println!(
        "  {BOLD}costcrew price database{RESET} {DIM}· bundled, {} models{RESET}",
        db.entries().len()
    );
    println!(
        "  {DIM}{} curated table · {} embedded snapshot · {} live cache{} · {} custom{RESET}",
        db.entries().iter().filter(|e| e.src == "curated").count(),
        db.embedded_count,
        db.cache_count,
        if db.embedded_fetched_at_ms > 0 {
            format!(" (snapshot {})", date_of(db.embedded_fetched_at_ms))
        } else {
            String::new()
        },
        db.custom_count,
    );
    println!("  {DIM}refresh anytime: {RESET}{CYAN}costcrew fetch-prices{RESET}");
    println!();
    if let Some(q) = query {
        println!("  {BOLD}{} matches for \"{q}\"{RESET}{DIM}  (--sort name|price|cheap, --top N, --json){RESET}", rows.len());
        println!();
    }
    println!(
        "  {:<44} {:>9} {:>9} {:>10}  {:<8}",
        "model", "in $/1M", "out $/1M", "cache $/1M", "source"
    );
    println!("  {}", "-".repeat(44 + 3 + 9 + 3 + 9 + 3 + 10 + 2 + 8));

    for e in rows.iter().take(top) {
        let (i, o, c) = (e.price.input, e.price.output, e.price.cache_read);
        let fmt = |v: f64| {
            if v > 0.0 {
                format!("{v:>9.3}")
            } else {
                "        ?".to_string()
            }
        };
        let src_col = match e.src.as_str() {
            "curated" => GREEN,
            "embedded" => CYAN,
            "cache" => YELLOW,
            _ => DIM,
        };
        println!(
            "  {:<44} {} {} {:>10}  ({src_col}{:<6}{DIM}){RESET}",
            truncate(&e.prefix, 44),
            fmt(i),
            fmt(o),
            fmt(c),
            e.src,
        );
    }

    if rows.len() > top {
        println!();
        println!(
            "  {DIM}... {} more. narrower filter, or --top {}. e.g.:{RESET}",
            rows.len() - top,
            rows.len()
        );
        println!(
            "  {DIM}  costcrew prices {}{RESET}",
            query.unwrap_or("anthropic")
        );
    }
    if rows.iter().take(top).any(|e| e.price.input == 0.0) {
        println!();
        println!(
            "  {DIM}? = free tier or unpriced. Unknown stays unknown — no invented prices.{RESET}"
        );
    }
    println!();
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "…"
    }
}

/// epoch ms → "Nov 10 2026" (civil-from-days, no chrono dependency)
fn date_of(ms: u64) -> String {
    if ms == 0 {
        return "undated".into();
    }
    let days = ms / 86_400_000;
    // Howard Hinnant's civil_from_days
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    const MON: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {} {}", MON[(m - 1) as usize], d, y)
}

#[cfg(test)]
mod tests {
    use super::date_of;
    #[test]
    fn epoch_to_civil() {
        assert_eq!(date_of(1_794_274_886_057), "Nov 10 2026");
        assert_eq!(date_of(0), "undated");
    }
}
