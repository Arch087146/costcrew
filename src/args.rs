//! Simple arg parsing — no clap, keep the binary lean.

#[derive(Debug)]
pub struct Args {
    pub cmd: Cmd,
    pub json: bool,
    pub since_days: u32,
    pub help: bool,
}

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Scan,
    Demo,
    Md,
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut args = Args {
        cmd: Cmd::Scan,
        json: false,
        since_days: 30,
        help: false,
    };

    let mut it = argv.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "scan" | "c" => {}
            "demo" | "d" => args.cmd = Cmd::Demo,
            "md" => args.cmd = Cmd::Md,
            "--json" => args.json = true,
            "--help" | "-h" | "help" => args.help = true,
            "--version" | "-V" | "-v" => {
                println!("costcrew {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
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
    costcrew [scan] [flags]     scan your machine for agent usage
    costcrew demo               show a sample report from synthetic data
    costcrew md > report.md     emit a shareable markdown report

FLAGS:
    --since <Nd|Nw|Nm>      look back N days/weeks/months (default 30d)
    --json                  machine-readable output
    -h, --help              this help
    -V, --version           version

SCANS:
    Claude Code transcripts   (~/.claude/**)
    OpenCode storage          (opencode data dir, legacy JSON)
    Codex CLI session logs    (~/.codex/**)",
        env!("CARGO_PKG_VERSION")
    );
}
