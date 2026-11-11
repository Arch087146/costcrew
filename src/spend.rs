//! `costcrew spend` — YOUR AI spending, as recorded by you.
//!
//! The tool never guesses here: you record what you actually paid
//! (subscription renewals, API credits, GPU rentals) and costcrew turns the
//! ledger into monthly totals and per-service breakdowns. It deliberately
//! complements the estimates from `scan` and the provider-truth from
//! `openrouter`: this is the number that leaves your bank account.
//!
//! Ledger: %LOCALAPPDATA%\costcrew\spend.json. No network. Nothing leaves disk.

use crate::fetch::cache_dir;
use crate::report::{BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW};
use serde_json::Value;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Entry {
    /// YYYY-MM-DD the spend happened (defaults to today)
    pub date: String,
    pub amount: f64,
    pub service: String,
    pub model: String,
    pub note: String,
}

// ---------- ledger IO ----------

fn ledger_path() -> std::path::PathBuf {
    cache_dir().join("spend.json")
}

pub fn all_entries() -> Vec<Entry> {
    load()
}

fn load() -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(ledger_path()) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    v.get("entries")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|e| {
                    Some(Entry {
                        date: e.get("date").and_then(Value::as_str)?.to_string(),
                        amount: e.get("amount").and_then(Value::as_f64)?,
                        service: e
                            .get("service")
                            .and_then(Value::as_str)
                            .unwrap_or("unspecified")
                            .to_string(),
                        model: e
                            .get("model")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        note: e
                            .get("note")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn save(entries: &[Entry]) -> Result<(), String> {
    let doc = serde_json::json!({
        "version": 1,
        "entries": entries.iter().map(|e| serde_json::json!({
            "date": e.date, "amount": e.amount,
            "service": e.service, "model": e.model, "note": e.note,
        })).collect::<Vec<_>>(),
    });
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(ledger_path(), doc.to_string()).map_err(|e| e.to_string())
}

// ---------- public commands ----------

/// `costcrew spend add <amount>` — record one payment.
pub fn add(args: &crate::args::Args) {
    let amount = match args.pos.first().map(|a| parse_amount(a)) {
        Some(Ok(a)) => a,
        Some(Err(e)) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
        None => {
            eprintln!("error: spend add needs an amount, e.g. costcrew spend add 19.99");
            eprintln!("       flags: --service \"Claude Pro\" --model opus --note \"monthly\"");
            std::process::exit(2);
        }
    };
    let date = args.at.clone().unwrap_or_else(today);
    match push_entry(Entry {
        date,
        amount,
        service: args.service.clone().unwrap_or_else(|| "unspecified".into()),
        model: args.model.clone().unwrap_or_default(),
        note: args.note.clone().unwrap_or_default(),
    }) {
        Ok(n) => println!(
            "  {GREEN}✔{RESET} recorded ${amount:.2} — {DIM}{} entries in the ledger{RESET}",
            n
        ),
        Err(e) => {
            eprintln!("error: can't write ledger: {e}");
            std::process::exit(1);
        }
    }
}

/// shared writer for the flag path and the interactive path
pub fn push_entry(e: Entry) -> Result<usize, String> {
    let mut entries = load();
    entries.push(e);
    save(&entries)?;
    Ok(entries.len())
}

/// menu-driven version of `spend add` — plain prompts, no flags
pub fn interactive_add() {
    println!();
    println!(
        "  {BOLD}record a payment{RESET} {DIM}· goes to {}{RESET}",
        ledger_path().display()
    );
    println!();
    let amount = match parse_amount(&prompt_of("amount $:")) {
        Ok(a) => a,
        Err(e) => {
            println!("  {RED}✘ {e}{RESET}");
            return;
        }
    };
    let service = {
        let s = prompt_of("service (e.g. Claude Pro):");
        if s.is_empty() {
            "unspecified".into()
        } else {
            s
        }
    };
    let model = prompt_of("model (enter to skip):");
    let note = prompt_of("note (enter to skip):");
    match push_entry(Entry {
        date: today(),
        amount,
        service,
        model,
        note,
    }) {
        Ok(_) => println!("  {GREEN}✔{RESET} recorded ${amount:.2}"),
        Err(e) => println!("  {RED}✘ can't write ledger: {e}{RESET}"),
    }
    println!();
}

fn prompt_of(label: &str) -> String {
    use std::io::Write;
    print!("  {CYAN}{label}{RESET} ");
    std::io::stdout().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

/// `costcrew spend` — the summary view.
pub fn print_spend(json: bool, lookback_days: u32) {
    let entries = load();
    let today = today();

    if json {
        use serde_json::json;
        let month_tot = total_in_month(&entries, &month_of(&today));
        let rolling = total_last_n_days(&entries, &today, lookback_days);
        let all: f64 = entries.iter().map(|e| e.amount).sum();
        let arr: Vec<_> = entries
            .iter()
            .map(|e| {
                json!({
                    "date": e.date, "amount": e.amount, "service": e.service,
                    "model": e.model, "note": e.note,
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "this_month_usd": (month_tot * 100.0).round() / 100.0,
                "last_n_days_usd": ((rolls_into(month_tot, rolling)) * 100.0).round() / 100.0,
                "all_time_usd": (all * 100.0).round() / 100.0,
                "entries": arr,
            })
        );
        return;
    }

    println!();
    println!(
        "  {BOLD}costcrew spend{RESET} {DIM}· your AI cash outlay (only what you recorded){RESET}"
    );
    println!();

    if entries.is_empty() {
        println!("  {YELLOW}nothing recorded yet.{RESET}");
        println!();
        println!("  {DIM}examples:{RESET}");
        println!("    {CYAN}costcrew spend add 20 --service \"Claude Pro\"{RESET}");
        println!("    {CYAN}costcrew spend add 25 --service \"OpenRouter\" --note \"credit top-up\"{RESET}");
        println!("    {CYAN}costcrew spend add 1.84 --service \"Vast.ai\" --model llama-70b --note \"2h @ $0.92/h\"{RESET}");
        println!();
        println!(
            "  {DIM}this ledger is what actually leaves your bank — `{CYAN}scan{DIM}` estimates\n             per-token usage, `{CYAN}openrouter{DIM}` reports key spend, this records the real bills.{RESET}"
        );
        println!();
        return;
    }

    let this_month = total_in_month(&entries, &month_of(&today));
    let rolling = total_last_n_days(&entries, &today, lookback_days);
    let all_time: f64 = entries.iter().map(|e| e.amount).sum();

    println!(
        "  {DIM}last {}d{RESET} {BOLD}${:>8.2}{RESET}   {DIM}this month{RESET} {BOLD}${:>8.2}{RESET}   {DIM}all-time{RESET} {BOLD}${:>8.2}{RESET}",
        lookback_days, rolling.min(this_month.max(rolling)), this_month, all_time
    );

    // by service — last N days
    let mut by_service: Vec<(String, f64)> = Vec::new();
    for e in &entries {
        if !in_last_n_days(&e.date, &today, lookback_days) {
            continue;
        }
        match by_service.iter_mut().find(|(s, _)| *s == e.service) {
            Some((_, v)) => *v += e.amount,
            None => by_service.push((e.service.clone(), e.amount)),
        }
    }
    by_service.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if !by_service.is_empty() {
        println!();
        println!("  {DIM}by service · last {}d{RESET}", lookback_days);
        let max = by_service.iter().fold(0.0f64, |m, (_, v)| m.max(*v));
        for (s, v) in &by_service {
            let bar = bar(*v, max);
            let col = value_color(*v, max);
            println!("    {:<18} {col}${:>7.2}{RESET} {bar}", truncate(s, 18), v);
        }
    }

    // by month, rolling 6
    let mut by_month: Vec<(String, f64)> = Vec::new();
    for e in &entries {
        let m = month_of(&e.date);
        match by_month.iter_mut().find(|(s, _)| *s == m) {
            Some((_, v)) => *v += e.amount,
            None => by_month.push((m, e.amount)),
        }
    }
    by_month.sort_by(|a, b| a.0.cmp(&b.0));
    let by_month = &by_month[by_month.len().saturating_sub(6)..];
    if !by_month.is_empty() {
        println!();
        println!("  {DIM}by month{RESET}");
        let max = by_month.iter().fold(0.0f64, |m, (_, v)| m.max(*v));
        for (m, v) in by_month {
            let bar = bar(*v, max);
            let mad = if *m == month_of(&today) { BOLD } else { "" };
            println!("    {mad}{m}{RESET}    {RED}${:>7.2}{RESET} {bar}", v);
        }
    }

    println!();
    println!(
        "  {DIM}estimate of the same window per-token: {RESET}{CYAN}costcrew scan{RESET}{DIM} · provider truth: {RESET}{CYAN}costcrew openrouter{RESET}{DIM}{RESET}"
    );
    println!();
}

fn rolls_into(_a: f64, b: f64) -> f64 {
    b
}

fn value_color(v: f64, max: f64) -> &'static str {
    let frac = if max > 0.0 { v / max } else { 0.0 };
    if frac >= 0.66 {
        RED
    } else if frac >= 0.33 {
        YELLOW
    } else {
        GREEN
    }
}

fn bar(v: f64, max: f64) -> String {
    if max <= 0.0 {
        return String::new();
    }
    let n = ((v / max) * 40.0).round() as usize;
    format!("{:<40}", "█".repeat(n))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>() + "…"
    }
}

/// "$1,234.56" / "19.99" / "0" → f64
pub fn parse_amount(s: &str) -> Result<f64, String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, '$' | ',' | ' '))
        .collect();
    cleaned
        .parse::<f64>()
        .map_err(|_| format!("not a dollar amount: {s}"))
        .and_then(|v| {
            if v.is_finite() && v >= 0.0 {
                Ok(v)
            } else {
                Err(format!("amount must be ≥ 0, got {s}"))
            }
        })
}

/// weekday-aware date helpers without chrono: days-from-civil (Hinnant)
fn days_from_civil(date: &str) -> i64 {
    let parts: Vec<_> = date
        .split('-')
        .filter_map(|p| p.parse::<i64>().ok())
        .collect();
    if parts.len() != 3 {
        return i64::MIN;
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m - 3).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe
}

pub fn today() -> String {
    let ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d: Duration| d.as_millis() as u64)
        .unwrap_or(0);
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    format!("{y:04}-{m:02}-{d:02}")
}

fn month_of(date: &str) -> String {
    date.chars().take(7).collect()
}

fn in_last_n_days(date: &str, today: &str, n: u32) -> bool {
    let (d, t) = (days_from_civil(date), days_from_civil(today));
    if d < i64::MAX / 2 {
        t >= d && (t - d) < n as i64
    } else {
        false
    }
}

fn total_in_month(entries: &[Entry], month: &str) -> f64 {
    entries
        .iter()
        .filter(|e| month_of(&e.date) == month)
        .map(|e| e.amount)
        .sum()
}

fn total_last_n_days(entries: &[Entry], today: &str, n: u32) -> f64 {
    entries
        .iter()
        .filter(|e| in_last_n_days(&e.date, today, n))
        .map(|e| e.amount)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_decode() {
        assert_eq!(parse_amount("19.99").unwrap(), 19.99);
        assert_eq!(parse_amount("$1,234.50").unwrap(), 1234.50);
        assert_eq!(parse_amount("0").unwrap(), 0.0);
        assert!(parse_amount("abc").is_err());
        assert!(parse_amount("-5").is_err());
    }

    #[test]
    fn civil_days_are_monotonic() {
        // sanity: crossing a month/year boundary keeps ordering
        let a = days_from_civil("2026-11-09");
        let b = days_from_civil("2026-11-10");
        let c = days_from_civil("2027-01-01");
        assert!(a < b && b < c);
        // ...and is consistent with days_from_civil convention on a known pair:
        // 2024-02-29 must exist after 2024-02-28 (leap day)
        assert!(days_from_civil("2024-02-28") < days_from_civil("2024-02-29"));
    }

    #[test]
    fn month_windows() {
        let e = |d: &str, a: f64| Entry {
            date: d.into(),
            amount: a,
            service: "test".into(),
            model: String::new(),
            note: String::new(),
        };
        let today = "2026-11-10";
        let entries = vec![
            e("2026-11-01", 20.0),
            e("2026-10-15", 100.0),
            e("2026-11-09", 5.5),
        ];
        assert_eq!(total_in_month(&entries, "2026-11"), 25.5);
        assert_eq!(total_in_month(&entries, "2026-10"), 100.0);
        assert_eq!(total_last_n_days(&entries, today, 30), 125.5); // Oct15 = 26d back, inside
                                                                   // 25-day window: back to Oct 16 → October entry excluded
        assert_eq!(total_last_n_days(&entries, today, 25), 25.5);
    }
}
