//! `costcrew budget` — "I have $X a month: what's the best model I can afford?"
//!
//! You name a monthly budget, optionally tick which tasks you actually use
//! AI for (or skip for the default agentic-coding mix), and costcrew walks
//! the arena leaderboard asking one question per model: can this month's
//! token appetite fit under the budget? The highest-Elo model that fits is
//! your pick. Unpriced models are excluded — never guessed.

use crate::board::BOARD;
use crate::price::{PriceDB, Pricing};
use crate::report::{BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW};

/// a usage task: monthly token appetite, in millions of tokens
pub struct Task {
    pub name: &'static str,
    pub in_m: f64,
    pub out_m: f64,
    pub cache_m: f64,
}

/// flat-rate subscription plans — completely different economics from token
/// billing: a seat includes the flagship models, rate-limited, no API.
/// List prices, approximate — check the vendor page.
const SUBS: &[(&str, f64, &str)] = &[
    (
        "ChatGPT Plus",
        20.0,
        "GPT-6 Astra / GPT-5.x — rate-limited, no API",
    ),
    (
        "Claude Pro",
        20.0,
        "Claude Opus / Sonnet — rate-limited, no API",
    ),
    (
        "GitHub Copilot Pro",
        10.0,
        "multi-model incl. GPT + Claude, in-IDE agents",
    ),
    (
        "Gemini (Google) Pro",
        20.0,
        "Gemini 4 / 3.8 Flash — rate-limited",
    ),
];

/// monthly token mixes per task, from real agent/session logs (approximate)
const TASKS: &[Task] = &[
    Task {
        name: "agentic coding (daily driver)",
        in_m: 6.0,
        out_m: 1.5,
        cache_m: 20.0,
    },
    Task {
        name: "chat & quick questions",
        in_m: 3.0,
        out_m: 0.8,
        cache_m: 0.5,
    },
    Task {
        name: "autocomplete / completions",
        in_m: 6.0,
        out_m: 0.6,
        cache_m: 3.0,
    },
    Task {
        name: "deep research / long docs",
        in_m: 10.0,
        out_m: 3.0,
        cache_m: 2.0,
    },
];

/// what one month of these tasks costs on this model's list price
pub fn monthly_cost(p: Pricing, tasks: &[&Task]) -> f64 {
    tasks
        .iter()
        .map(|t| t.in_m * p.input + t.out_m * p.output + t.cache_m * p.cache_read)
        .sum()
}

struct Row {
    idx: usize, // index into BOARD
    cost: f64,
}

/// CLI entry: `costcrew budget 20` or `costcrew budget 20 1,3`
/// (amount in pos[0], task numbers "1,3" in pos[1]).
pub fn run(db: &PriceDB, pos: &[String], json: bool) {
    let amount = match pos.first().map(|s| s.trim().trim_start_matches('$')) {
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v > 0.0 => v,
            _ => {
                eprintln!("error: budget must be a positive number, got {s:?}");
                std::process::exit(2);
            }
        },
        None => return interactive_plan(db),
    };
    let picks = parse_task_list(pos.get(1).map(|s| s.as_str()).unwrap_or(""));
    plan(db, amount, &picks, json);
}

/// interactive entry (menu item 3): asks the budget, then the tasks
pub fn interactive_plan(db: &PriceDB) {
    println!();
    println!("  {BOLD}costcrew budget planner{RESET}");
    println!();
    let amount = loop {
        let s = crate::ui::prompt("monthly budget in USD (e.g. 20)");
        match s.trim().trim_start_matches('$').parse::<f64>() {
            Ok(v) if v > 0.0 => break v,
            _ => println!("  {DIM}enter a positive number{RESET}"),
        }
    };
    println!();
    println!(
        "  {CYAN}what do you use AI for?{RESET} {DIM}(comma numbers, enter = just coding){RESET}"
    );
    for (i, t) in TASKS.iter().enumerate() {
        println!("    {} {}", i + 1, t.name);
    }
    let s = crate::ui::prompt("tasks");
    let picks = if s.trim().is_empty() {
        vec![0]
    } else {
        parse_task_list(&s)
    };
    plan(db, amount, &picks, false);
}

fn parse_task_list(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for tok in s.split(',') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(n) = t.parse::<usize>() {
            if (1..=TASKS.len()).contains(&n) && !out.contains(&(n - 1)) {
                out.push(n - 1);
            }
        }
    }
    if out.is_empty() {
        out.push(0); // default: agentic coding
    }
    out
}

pub fn plan(db: &PriceDB, budget: f64, picks: &[usize], json: bool) {
    let tasks: Vec<&Task> = picks.iter().map(|&i| &TASKS[i]).collect();

    // price every board model for this task mix
    let mut rows: Vec<Row> = Vec::new();
    let mut unpriced = 0usize;
    for (i, b) in BOARD.iter().enumerate() {
        match db.lookup(b.prefix) {
            Some(p) if p.input + p.output > 0.0 => {
                rows.push(Row {
                    idx: i,
                    cost: monthly_cost(p, &tasks),
                });
            }
            _ => unpriced += 1,
        }
    }
    rows.sort_by(|a, b| {
        a.cost
            .partial_cmp(&b.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let affordable: Vec<&Row> = rows.iter().filter(|r| r.cost <= budget).collect();
    let best = affordable.iter().max_by_key(|r| BOARD[r.idx].elo as u64);
    let names = tasks.iter().map(|t| t.name).collect::<Vec<_>>().join(" + ");

    if json {
        use serde_json::json;
        let arr: Vec<_> = affordable
            .iter()
            .rev()
            .take(10)
            .map(|r| {
                let b = &BOARD[r.idx];
                json!({
                    "model": b.display.trim(),
                    "prefix": b.prefix,
                    "arena_elo": b.elo,
                    "est_monthly_cost": (r.cost * 100.0).round() / 100.0,
                })
            })
            .collect();
        let best_json = best.map(|r| {
            let b = &BOARD[r.idx];
            json!({
                "model": b.display.trim(),
                "prefix": b.prefix,
                "arena_elo": b.elo,
                "est_monthly_cost": (r.cost * 100.0).round() / 100.0,
                "budget": budget,
            })
        });
        println!(
            "{}",
            json!({
                "budget": budget,
                "tasks": names,
                "best": best_json,
                "affordable_top": arr,
                "unpriced_excluded": unpriced,
            })
        );
        return;
    }

    println!();
    println!(
        "  {BOLD}costcrew budget{RESET} {DIM}· ${:.2}/month · {names}{RESET}",
        budget
    );
    println!();

    match best {
        Some(r) => {
            let b = &BOARD[r.idx];
            let pct = 100.0 * r.cost / budget;
            println!(
                "  {GREEN}{BOLD}best you can afford:{RESET} {BOLD}{}{RESET} {DIM}(elo {:.0}, {} votes){RESET}",
                b.display.trim(),
                b.elo,
                crate::board::format_votes(b.votes)
            );
            println!(
                "  {CYAN}${:.2}{RESET} estimated monthly cost {DIM}· {:.0}% of budget{RESET}",
                r.cost, pct
            );

            println!();
            println!(
                "  {:>3} {:<30} {:>5} {:>10}",
                "#", "affordable, best first", "elo", "$/month"
            );
            println!("  {}", "-".repeat(3 + 1 + 30 + 1 + 5 + 1 + 10));
            let mut by_elo = affordable.clone();
            by_elo.sort_by_key(|row| std::cmp::Reverse(BOARD[row.idx].elo as u64));
            for (i, row) in by_elo.iter().take(8).enumerate() {
                let b = &BOARD[row.idx];
                let marker = if row.idx == r.idx { GREEN } else { DIM };
                println!(
                    "  {marker}{:>3} {:<30} {:>5.0} {:>10.2}{RESET}",
                    i + 1,
                    b.display,
                    b.elo,
                    row.cost,
                    marker = marker
                );
            }

            // the closest step up — CHEAPEST model that beats the pick
            let best_elo = BOARD[r.idx].elo;
            if let Some(up) = rows
                .iter()
                .find(|row| row.cost > budget && BOARD[row.idx].elo > best_elo)
            {
                let b = &BOARD[up.idx];
                println!();
                println!(
                    "  {YELLOW}stretch:{RESET} ${:.2}/mo buys {} {DIM}(elo {:.0}){RESET}",
                    up.cost,
                    b.display.trim(),
                    b.elo
                );
            }
        }
        None => {
            println!("  {RED}nothing fits ${:.2}/month.{RESET}", budget);
            if let Some(cheapest) = rows.first() {
                let b = &BOARD[cheapest.idx];
                println!(
                    "  the cheapest option is {} at {CYAN}${:.2}{RESET}/month",
                    b.display.trim(),
                    cheapest.cost
                );
            }
        }
    }

    if unpriced > 0 {
        println!();
        println!(
            "  {DIM}{unpriced} board models excluded — no price in the database, and ? is never guessed.{RESET}"
        );
    }

    // flat-rate subscriptions: a ~$20 seat can include models that would cost
    // 5-10x this budget on pay-per-token rails — say so, it changes the answer
    let subs: Vec<&(&str, f64, &str)> = SUBS
        .iter()
        .filter(|(_, price, _)| *price <= budget)
        .collect();
    if !subs.is_empty() {
        println!();
        println!("  {BOLD}flat-rate subscriptions that fit{RESET} {DIM}(seat ≠ tokens: rate-limited, no API){RESET}");
        for (name, price, includes) in subs {
            println!("    {CYAN}{name}{RESET} {DIM}~${price:.0}/mo{RESET} — {includes}");
        }
    }

    println!();
    println!(
        "  {DIM}estimates = real list prices × task token mixes (in/out/cache per month).{RESET}"
    );
    println!(
        "  {DIM}quality = LMArena Coding Elo snapshot. flat subs above are list prices{RESET}"
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_is_linear_in_tokens() {
        let t = &TASKS[0];
        let p = Pricing {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 0.0,
            per_hour: 0.0,
        };
        // 6M in + 1.5M out + 20M cache
        assert!((monthly_cost(p, &[t]) - (6.0 * 3.0 + 1.5 * 15.0 + 20.0 * 0.3)).abs() < 1e-9);
    }

    #[test]
    fn cheap_strong_model_wins_a_small_budget() {
        let db = PriceDB::curated();
        // deepseek-chat: 6*0.27 + 1.5*1.1 + 20*0.07 = 4.67/mo on the coding mix
        let p = db.lookup("deepseek-chat").expect("curated has deepseek");
        let cost = monthly_cost(p, &[&TASKS[0]]);
        assert!((cost - 4.67).abs() < 1e-9);
        // opus-class pricing must NOT fit a $10 budget on the coding mix
        let p = db.lookup("claude-opus-4").expect("curated has opus");
        assert!(monthly_cost(p, &[&TASKS[0]]) > 10.0);
    }
}
