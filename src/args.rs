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
}

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Scan,
    Demo,
    Md,
    Advise,
    Catalog,
    Leaderboard,
    OpenRouter,
    Fetch,
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut cmd = Cmd::Scan;
    let mut seen_cmd = false;
    let mut args = Args {
        cmd: Cmd::Scan,
        json: false,
        since_days: 30,
        help: false,
        key: None,
        prices: None,
        sort: None,
        top: 25,
    };

    let mut it = argv.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "scan" | "c" if !seen_cmd => {
                cmd = Cmd::Scan;
                seen_cmd = true;
            }
            "demo" | "d" if !seen_cmd => {
                cmd = Cmd::Demo;
                seen_cmd = true;
            }
            "md" if !seen_cmd => {
                cmd = Cmd::Md;
                seen_cmd = true;
            }
            "advise" | "a" if !seen_cmd => {
                cmd = Cmd::Advise;
                seen_cmd = true;
            }
            "leaderboard" | "lb" | "board" if !seen_cmd => {
                cmd = Cmd::Leaderboard;
                seen_cmd = true;
            }
            "catalog" if !seen_cmd => {
                cmd = Cmd::Catalog;
                seen_cmd = true;
            }
            "openrouter" | "or" if !seen_cmd => {
                cmd = Cmd::OpenRouter;
                seen_cmd = true;
            }
            "fetch-prices" if !seen_cmd => {
                cmd = Cmd::Fetch;
                seen_cmd = true;
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
                } else {
                    return Err(format!("unknown argument: {a}\ntry `costcrew --help`"));
                }
            }
        }
    }
    args.cmd = cmd;
    if args.help {
        print_help();
        std::process::exit(0);
    }
    Ok(args)
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
        "costcrew {} — what do your AI agents actually cost?

USAGE:
    costcrew [scan] [flags]      scan local agent logs, price them
    costcrew advise              cheaper-model verdicts (could you have paid less?)
    costcrew md > report.md      shareable markdown report
    costcrew demo                sample report from synthetic data
    costcrew openrouter          ACTUAL spend for an OpenRouter API key
                                 (env OPENROUTER_API_KEY or --key sk-or-v1-...)
    costcrew fetch-prices        pull OpenRouter's live price list (~460 models)
                                 cached at %LOCALAPPDATA%\\costcrew — all future
                                 scans price against it automatically
    costcrew leaderboard         arena-style Elo leaderboard, priced for
                                 agentic token mixes:
                                 --sort elo|value|price  --top N  --json
    costcrew catalog             which models each agent service offers

FLAGS:
    --since <Nd|Nw|Nm>           lookback for scan/advise (default 30d)
    --prices <file.json>         custom price overrides, e.g. a rented GPU:
                                 [{{\"prefix\":\"local/llama-3\",\"per_hour\":2.19}}]
                                 per_hour bills wall-clock (opencode logs the ms)
    --json                       machine-readable output
    -h, --help                   this help
    -V, --version                version

SCANS (read-only):
    Claude Code transcripts      (~/.claude/**)
    OpenCode storage             (opencode.db sqlite + legacy JSON)
    Codex CLI session logs       (~/.codex/**)
    Kilo Code / Cline / Roo Code (VS Code globalStorage tasks)
    Aider                        (cost lines in .aider*.history.md)",
        env!("CARGO_PKG_VERSION")
    );
}
