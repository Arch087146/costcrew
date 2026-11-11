mod advise;
mod args;
mod board;
mod catalog;
mod demo;
mod fetch;
mod price;
mod prices;
mod report;
mod scan;
mod spend;
mod ui;

use args::{parse, Args, Cmd};
use price::PriceDB;
use report::{print_markdown, print_table, Totals, BOLD, DIM, RESET};
use scan::{aggregate, collect};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // bare `costcrew` = the interactive app (splash + arrow-key menu).
    // any argument at all = classic flag mode for scripting.
    if argv.is_empty() {
        ui::run_app();
        return;
    }

    let args = match parse(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    flow(args);
}

/// one dispatch point, shared by the flag CLI and the interactive menu
pub fn flow(args: Args) {
    match args.cmd {
        Cmd::Overview => overview(&args),
        Cmd::Prices => {
            let mut db = PriceDB::builtin();
            fetch::cached_into_db(&mut db);
            if let Some(path) = &args.prices {
                load_custom(&mut db, path);
            }
            let sort = args.sort.as_deref().unwrap_or("name").to_string();
            let query = args.pos.first().cloned();
            prices::print_prices(&db, query.as_deref(), &sort, args.top, args.json);
        }
        Cmd::Spend => spend::print_spend(args.json, args.since_days),
        Cmd::SpendAdd => spend::add(&args),
        Cmd::Demo => {
            let db = PriceDB::builtin();
            run(demo::demo_rows(&db), args, 7, &db);
        }
        Cmd::Catalog => catalog::print_catalog(),
        Cmd::Leaderboard => {
            let mut db = PriceDB::builtin();
            fetch::cached_into_db(&mut db);
            if let Some(path) = &args.prices {
                load_custom(&mut db, path);
            }
            let sort = match &args.sort {
                Some(s) => match board::parse_sort(s) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(2);
                    }
                },
                None => board::Sort::Elo,
            };
            board::print_board(&db, sort, args.top, args.json);
        }
        Cmd::OpenRouter => {
            run_openrouter(&args);
        }
        Cmd::Fetch => {
            println!("fetching OpenRouter price list ...");
            match fetch::force_refresh() {
                Ok(n) => println!(
                    "cached {} models to {}\nnext `costcrew` run prices against them automatically",
                    n,
                    fetch::cache_dir().join("openrouter-prices.json").display()
                ),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Cmd::Scan | Cmd::Md | Cmd::Advise => {
            let mut db = PriceDB::builtin();
            fetch::cached_into_db(&mut db);
            if let Some(path) = &args.prices {
                load_custom(&mut db, path);
            }
            let events = collect(args.since_days);
            if events.is_empty() {
                if args.cmd == Cmd::Advise {
                    // advise is understood without a scan — show it on demo data
                    let db = PriceDB::builtin();
                    println!("  {DIM}(no local usage found — showing advise on demo data){RESET}");
                    advise::print_advise(&demo::demo_rows(&db), &db, 7);
                    return;
                }
                eprintln!("no agent usage found on this machine.");
                eprintln!("scanned ~/.claude, opencode data dirs, ~/.codex, vscode globalStorage, aider history");
                eprintln!("try `costcrew demo` to see a sample report.");
                std::process::exit(1);
            }
            let days = args.since_days;
            let rows = aggregate(&events, &db);
            if args.cmd == Cmd::Advise && !rows.iter().any(|r| r.known_price) {
                // advise is understood without priced scans — show it on demo data
                println!(
                    "{DIM}(no priced usage found locally — showing advise on demo data){RESET}"
                );
                advise::print_advise(&demo::demo_rows(&db), &db, 7);
                return;
            }
            run(rows, args, days, &db);
        }
    }
}

fn run(rows: Vec<scan::Row>, args: args::Args, since_days: u32, db: &PriceDB) {
    let total_cost: f64 = rows.iter().filter(|r| r.known_price).map(|r| r.cost).sum();
    let total_requests: u64 = rows.iter().map(|r| r.requests).sum();
    let unknown_models = rows.iter().filter(|r| !r.known_price).count();
    let totals = Totals {
        total_cost,
        total_requests,
        unknown_models,
    };

    if args.json {
        print_json(&rows, &totals);
        return;
    }

    match args.cmd {
        Cmd::Md => print_markdown(&rows, since_days, &totals),
        Cmd::Advise => advise::print_advise(&rows, db, since_days),
        _ => print_table(&rows, since_days, &totals),
    }
}

/// Bare `costcrew` — the two things the tool is about:
/// the bundled price database, and YOUR spend on AI.
fn overview(args: &args::Args) {
    use report::{CYAN, GREEN, YELLOW};
    let mut db = PriceDB::builtin();
    fetch::cached_into_db(&mut db);
    if let Some(path) = &args.prices {
        load_custom(&mut db, path);
    }
    let total = db.entries().len();

    println!();
    println!("  {BOLD}costcrew{RESET} {DIM}· the AI coding price database + your spend{RESET}");
    println!();

    println!("  {BOLD}THE DATABASE{RESET} {GREEN}{total} models priced{RESET} {DIM}(bundled — works offline){RESET}");
    println!("  {DIM}per-model: input / output / cache rates per 1M tokens{RESET}");
    println!();
    println!(
        "    {CYAN}costcrew prices{RESET}            browse everything {DIM}({} curated + {} embedded){RESET}",
        db.entries().iter().filter(|e| e.src == "curated").count(),
        db.embedded_count
    );
    println!("    {CYAN}costcrew prices claude{RESET}     filter by name {DIM}(--sort price|cheap|name){RESET}");
    println!("    {CYAN}costcrew leaderboard{RESET}       arena Elo, priced: elo-per-dollar value ranking");
    println!("    {CYAN}costcrew catalog{RESET}           which agent ships with which models");
    println!(
        "    {CYAN}costcrew fetch-prices{RESET}      refresh from OpenRouter live (~{} models)",
        db.embedded_count.max(465)
    );

    // ---- your spend ----
    let entries = spend_load_entries();
    let today = today_str();
    let this_month = entries
        .iter()
        .filter(|e| e.date.chars().take(7).collect::<String>() == today[..7])
        .map(|e| e.amount)
        .sum::<f64>();
    println!();
    println!("  {BOLD}YOUR SPEND ON AI{RESET}");
    if entries.is_empty() {
        println!(
            "  {YELLOW}nothing recorded yet{RESET} {DIM}— log what you actually pay (subscriptions, credits, GPUs):{RESET}"
        );
        println!("    {CYAN}costcrew spend add 20 --service \"Claude Pro\"{RESET}");
    } else {
        println!(
            "  {DIM}this month{RESET} {BOLD}${this_month:>7.2}{RESET} {DIM}· {} entries{RESET}",
            entries.len()
        );
    }
    println!("    {CYAN}costcrew spend{RESET}            monthly totals + per-service breakdown");
    println!("    {CYAN}costcrew spend add 25{RESET}     record a payment {DIM}--service --model --note --at{RESET}");
    println!("    {CYAN}costcrew openrouter{RESET}       actual spend per model from your API key");

    // ---- optional estimates ----
    println!();
    println!("  {DIM}optional: {RESET}{DIM}estimate per-token usage from local logs → {RESET}{CYAN}costcrew scan{RESET}");
    println!(
        "  {DIM}cheaper-model advice on your own token mix → {RESET}{CYAN}costcrew advise{RESET}"
    );
    if args.json {
        use serde_json::json;
        println!(
            "{}",
            json!({
                "database_models": total,
                "curated": db.entries().iter().filter(|e| e.src == "curated").count(),
                "embedded": db.embedded_count,
                "cache": db.cache_count,
                "custom": db.custom_count,
                "spend_this_month_usd": (this_month * 100.0).round() / 100.0,
                "spend_entries": entries.len(),
            })
        );
    }
    println!();
}

fn spend_load_entries() -> Vec<spend::Entry> {
    spend::all_entries()
}

fn today_str() -> String {
    spend::today()
}

fn run_openrouter(args: &args::Args) {
    let key = args
        .key
        .clone()
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("error: set OPENROUTER_API_KEY or pass --key sk-or-v1-...");
            std::process::exit(2);
        });

    println!("querying OpenRouter activity (limit 200) ...");
    match fetch::activity(&key) {
        Ok(rows) if rows.is_empty() => println!("no recorded spend — nothing on this key yet."),
        Ok(rows) => {
            println!();
            println!("  {BOLD}costcrew{RESET} {DIM}· OPENROUTER actual spend (API key){RESET}");
            println!();
            println!("  {:>40}  {:>6}  {:>10}", "model", "reqs", "cost usd");
            println!("  {}:{}", "-".repeat(40), "-".repeat(19));
            let total: f64 = rows.iter().map(|(_, _, c)| c).sum();
            for (model, reqs, cost) in &rows {
                println!(
                    "  {:>40}  {:>6}  {:>10.4}",
                    if model.len() > 40 {
                        &model[..40]
                    } else {
                        model
                    },
                    reqs,
                    cost
                );
            }
            println!("  {:>40}  {:>6}  {:>10.4}", "TOTAL", "", total);
            println!();
            println!("  {DIM}this is act bill-truth from the provider — the scan is the estimate.{RESET}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn load_custom(db: &mut PriceDB, path: &str) {
    use serde_json::Value;
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("error: can't read {path}");
        std::process::exit(2);
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        eprintln!("error: {path} is not valid JSON");
        std::process::exit(2);
    };
    let n = match v.get("prices") {
        Some(list) => load_list(db, list),
        None => {
            if v.is_array() {
                load_list(db, &v)
            } else {
                0
            }
        }
    };
    db.custom_count = n;
    println!(
        "using {n} custom price entr{} from {path}",
        if n == 1 { "y" } else { "ies" }
    );
}

fn load_list(db: &mut PriceDB, list: &serde_json::Value) -> usize {
    let mut n = 0;
    if let Some(items) = list.as_array() {
        for it in items {
            let prefix = match it.get("prefix").and_then(serde_json::Value::as_str) {
                Some(p) => p,
                None => continue,
            };
            let get = |k: &str| {
                it.get(k)
                    .and_then(serde_json::Value::as_f64)
                    .or_else(|| {
                        it.get(k)
                            .and_then(serde_json::Value::as_str)
                            .and_then(|s| s.trim().parse().ok())
                    })
                    .unwrap_or(0.0)
            };
            db.push(
                prefix,
                "custom",
                price::Pricing {
                    input: get("input"),
                    output: get("output"),
                    cache_read: get("cache_read"),
                    cache_write: get("cache_write"),
                    per_hour: get("per_hour"),
                },
            );
            n += 1;
        }
    }
    n
}

fn print_json(rows: &[scan::Row], t: &Totals) {
    use serde_json::json;
    let arr: Vec<_> = rows
        .iter()
        .map(|r| {
            json!({
                "provider": r.provider,
                "model": r.model,
                "requests": r.requests,
                "input_tokens": r.input_tokens,
                "output_tokens": r.output_tokens,
                "cache_tokens": r.cache_tokens,
                "duration_secs": (r.duration_secs * 10.0).round() / 10.0,
                "cost_usd": if r.known_price { serde_json::Value::from((r.cost * 10_000.0).round() / 10_000.0) } else { serde_json::Value::Null },
            })
        })
        .collect();
    println!(
        "{}",
        json!({
            "total_cost_usd": (t.total_cost * 100.0).round() / 100.0,
            "requests": t.total_requests,
            "unpriced_models": t.unknown_models,
            "rows": arr,
        })
    );
}
