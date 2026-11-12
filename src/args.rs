//! Simple arg parsing — no clap, keep the binary lean.

#[derive(Debug)]
pub struct Args {
    pub cmd: Cmd,
    pub json: bool,
    pub since_days: u32,
    pub help: bool,
    pub key: Option<String>,
    pub prices: Option<String>,
    pub sort: Option<String>,
    pub top: usize,
    /// positional words after the command ("prices claude", "spend add 20")
    pub pos: Vec<String>,
    pub service: Option<String>,
    pub model: Option<String>,
    pub note: Option<String>,
    pub at: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Overview,
    Scan,
    Demo,
    Md,
    Advise,
    Catalog,
    Leaderboard,
    Budget,
    OpenRouter,
    Prices,
    Spend,
    SpendAdd,
    Fetch,
}

/// two-word commands, checked before single-word matching
pub fn split_cmd(argv: &[String]) -> Option<(Cmd, &[String])> {
    if argv.len() >= 2 {
        match (argv[0].as_str(), argv[1].as_str()) {
            ("spend", "add") | ("s", "add") => return Some((Cmd::SpendAdd, &argv[2..])),
            _ => {}
        }
    }
    None
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    if let Some((sub, rest)) = split_cmd(argv) {
        let mut args = def(sub);
        parse_into(&mut args, rest)?;
        return Ok(args);
    }
    let mut args = def(Cmd::Overview);
    parse_inner(&mut args, argv, &mut false)?; // commands may set cmd; default overview
    if args.help {
        print_help();
        std::process::exit(0);
    }
    Ok(args)
}

fn def(cmd: Cmd) -> Args {
    Args {
        cmd,
        json: false,
        since_days: 30,
        help: false,
        key: None,
        prices: None,
        sort: None,
        top: 25,
        pos: Vec::new(),
        service: None,
        model: None,
        note: None,
        at: None,
    }
}

fn parse_into(args: &mut Args, argv: &[String]) -> Result<(), String> {
    let mut seen_cmd = true; // command already consumed ("spend add X")
    parse_inner(args, argv, &mut seen_cmd)
}

pub fn parse_inner(args: &mut Args, argv: &[String], seen_cmd: &mut bool) -> Result<(), String> {
    let mut it = argv.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "prices" | "db" if !*seen_cmd => {
                args.cmd = Cmd::Prices;
                *seen_cmd = true;
            }
            "spend" if !*seen_cmd => {
                args.cmd = Cmd::Spend;
                *seen_cmd = true;
            }
            "scan" | "c" if !*seen_cmd => {
                args.cmd = Cmd::Scan;
                *seen_cmd = true;
            }
            "demo" | "d" if !*seen_cmd => {
                args.cmd = Cmd::Demo;
                *seen_cmd = true;
            }
            "md" if !*seen_cmd => {
                args.cmd = Cmd::Md;
                *seen_cmd = true;
            }
            "advise" | "a" if !*seen_cmd => {
                args.cmd = Cmd::Advise;
                *seen_cmd = true;
            }
            "leaderboard" | "lb" | "board" if !*seen_cmd => {
                args.cmd = Cmd::Leaderboard;
                *seen_cmd = true;
            }
            "budget" | "plan" if !*seen_cmd => {
                args.cmd = Cmd::Budget;
                *seen_cmd = true;
            }
            "catalog" if !*seen_cmd => {
                args.cmd = Cmd::Catalog;
                *seen_cmd = true;
            }
            "openrouter" | "or" if !*seen_cmd => {
                args.cmd = Cmd::OpenRouter;
                *seen_cmd = true;
            }
            "fetch-prices" if !*seen_cmd => {
                args.cmd = Cmd::Fetch;
                *seen_cmd = true;
            }
            "--json" => args.json = true,
            "--sort" => {
                args.sort = Some(it.next().ok_or("--sort needs a value")?.to_string());
            }
            "--top" => {
                let v = it.next().ok_or("--top needs a number")?;
                args.top = v
                    .parse()
                    .map_err(|_| format!("--top must be a number, got {v}"))?;
            }
            "--service" => {
                args.service = Some(it.next().ok_or("--service needs a value")?.to_string());
            }
            "--model" => {
                args.model = Some(it.next().ok_or("--model needs a value")?.to_string());
            }
            "--note" => {
                args.note = Some(it.next().ok_or("--note needs a value")?.to_string());
            }
            "--at" => {
                args.at = Some(it.next().ok_or("--at needs YYYY-MM-DD")?.to_string());
            }
            "--help" | "-h" | "help" => args.help = true,
            "--version" | "-V" | "-v" => {
                println!("costcrew {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--key" => {
                args.key = Some(it.next().ok_or("--key needs a value")?.to_string());
            }
            "--prices" => {
                args.prices = Some(it.next().ok_or("--prices needs a path")?.to_string());
            }
            "--since" => {
                let v = it.next().ok_or("--since needs a value like 7d")?;
                args.since_days = parse_since(v)?;
            }
            _ => {
                if let Some(rest) = a.strip_prefix("--since=") {
                    args.since_days = parse_since(rest)?;
                } else if a.starts_with('-') {
                    return Err(format!("unknown argument: {a}\ntry `costcrew --help`"));
                } else if a == "add" && args.cmd == Cmd::Spend {
                    // tolerate `costcrew spend add 20` ordering variants
                } else {
                    args.pos.push(a.to_string());
                }
            }
        }
    }
    Ok(())
}

fn parse_since(v: &str) -> Result<u32, String> {
    let v = v.trim().to_lowercase();
    let (num, unit) = v.split_at(v.len().saturating_sub(1));
    let n: u32 = num.parse().map_err(|_| format!("bad --since value: {v}"))?;
    match unit {
        "d" => Ok(n.max(1)),
        "w" => Ok(n * 7),
        "m" => Ok(n * 30),
        _ => Err(format!("--since units are d/w/m, got: {v}")),
    }
}

pub fn print_help() {
    println!(
        "costcrew {} — the AI coding price database, plus your spend

USAGE — THE DATABASE:
    costcrew                     opens the interactive app (splash + menu);
                                 any argument instead uses classic flag mode
    costcrew prices [filter]     browse {} models (bundled, offline):
                                 every input/output/cache rate per 1M tokens
                                 --sort name|price|cheap  --top N  --json
    costcrew leaderboard         arena Elo ranked with prices:
                                 --sort elo|value|price  --top N  --json
    costcrew budget 20           best model you can afford for $20/month:
                                 asks what tasks you use AI for (or defaults
                                 to agentic coding); `costcrew budget 20 1,3`
                                 pre-picks the task mix from the list
    costcrew catalog             which agent ships with which models
    costcrew fetch-prices        refresh the DB from OpenRouter live
                                 (cached at %LOCALAPPDATA%\\costcrew)

USAGE — YOUR SPEND:
    costcrew spend               what YOU pay for AI: monthly totals,
                                 per-service breakdown (you record it)
    costcrew spend add 19.99     record a payment:
                                 --service \"Claude Pro\" --model opus
                                 --note \"monthly\" --at 2026-11-10
    costcrew openrouter          actual per-model spend from an API key
                                 (env OPENROUTER_API_KEY or --key sk-or-v1-...)

USAGE — OPTIONAL ESTIMATES:
    costcrew [scan]              estimate per-token cost from local agent logs
    costcrew advise              could you have used a cheaper model? verdicts
    costcrew md > report.md      shareable markdown report
    costcrew demo                sample report from synthetic data

FLAGS:
    --since <Nd|Nw|Nm>           lookback for scan/advise/spend (default 30d)
    --prices <file.json>         custom price overrides, e.g. a rented GPU:
                                 [{{\"prefix\":\"local/llama-3\",\"per_hour\":2.19}}]
                                 per_hour bills wall-clock (opencode logs the ms)
    --json                       machine-readable output
    -h, --help                   this help

UNKNOWN PRICES STAY '?':
    the database never invents a number — a free/unpriced model shows ?",
        env!("CARGO_PKG_VERSION"),
        509,
    );
}
