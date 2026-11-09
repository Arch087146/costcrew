mod args;
mod demo;
mod price;
mod report;
mod scan;

use args::{parse, Cmd};
use report::{print_markdown, print_table, Totals};
use scan::{aggregate, collect};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    match args.cmd {
        Cmd::Demo => run(demo::demo_rows(), args, 7),
        Cmd::Scan | Cmd::Md => {
            let events = collect(args.since_days);
            if events.is_empty() {
                eprintln!("no agent usage found on this machine.");
                eprintln!("scanned ~/.claude, opencode data dirs, ~/.codex");
                eprintln!("try `costcrew demo` to see a sample report.");
                std::process::exit(1);
            }
            let days = args.since_days;
            run(aggregate(&events), args, days);
        }
    }
}

fn run(rows: Vec<scan::Row>, args: args::Args, since_days: u32) {
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
        _ => print_table(&rows, since_days, &totals),
    }
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
