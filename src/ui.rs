//! `costcrew` interactive mode — the fancy bit.
//!
//! Bare `costcrew` opens this instead of dumping a table: a big splash, a
//! spinner while the bundled database loads, then an arrow-key menu. All the
//! flags keep working for scripting — this is the human front door.
//!
//! Rendering is plain ANSI (same style constants as the rest of the app);
//! crossterm is used only to capture key presses. Raw mode is enabled only
//! while waiting for a key, so every view can keep using plain `println!`.

use crate::args::{Args, Cmd};
use crate::price::PriceDB;
use crate::report::{BOLD, CYAN, DIM, RESET, YELLOW};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::Write;

/// ANSI 256-color cyan→blue gradient rows for the banner
const GRAD: [&str; 5] = [
    "\x1b[38;5;51m",
    "\x1b[38;5;45m",
    "\x1b[38;5;39m",
    "\x1b[38;5;33m",
    "\x1b[38;5;27m",
];

/// "COST / CREW" in solid blocks (ANSI-shadow double-line glyphs render as
/// tofu in plain conhost; solid █ renders everywhere)
#[rustfmt::skip]
const BANNER: &[&str] = &[
    "█████ █████ █████ █████",
    "█     █   █ █       █  ",
    "█     █   █ █████   █  ",
    "█     █   █     █   █  ",
    "█████ █████ █████   █  ",
    "█████ █████ █   █ █   █",
    "█   █ █     █   █ █   █",
    "█████ ████  █ █ █ █ █ █",
    "█   █ █     █ █ █ █ █ █",
    "█   █ █████ ██ ██ ██ ██",
];

struct Item {
    name: &'static str,
    hint: &'static str,
}

const ITEMS: [Item; 10] = [
    Item {
        name: "price database",
        hint: "every model, bundled, offline — browse & filter",
    },
    Item {
        name: "your spend",
        hint: "what YOU pay for AI: monthly totals, per service",
    },
    Item {
        name: "add a payment",
        hint: "record what you just paid (sub, credits, GPU)",
    },
    Item {
        name: "leaderboard",
        hint: "arena Elo ranked with prices — elo-per-dollar",
    },
    Item {
        name: "agent catalog",
        hint: "which agent ships with which models",
    },
    Item {
        name: "openrouter actuals",
        hint: "provider-reported spend from your API key",
    },
    Item {
        name: "refresh prices",
        hint: "update the database from OpenRouter live",
    },
    Item {
        name: "scan local logs",
        hint: "optional: estimate per-token from agent logs",
    },
    Item {
        name: "cheaper-model advice",
        hint: "could you have used a cheaper model?",
    },
    Item {
        name: "demo",
        hint: "sample report, no data needed",
    },
];

const FOOTER: &str = "↑/↓ move · Enter select · 1-9,0 jump · q quit";

pub fn run_app() {
    let mut out = std::io::stdout();
    print!("\x1b[2J\x1b[3J\x1b[H\x1b[?25l");
    splash(&mut out);

    let mut db = PriceDB::builtin();
    crate::fetch::cached_into_db(&mut db);

    let mut sel = 0usize;
    loop {
        draw_menu(&mut out, &db, sel);
        let key = match wait_key() {
            Some(k) => k,
            None => break,
        };
        match key.code {
            KeyCode::Esc => break,
            KeyCode::Char('q') | KeyCode::Char('Q') => break,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Up => sel = sel.wrapping_sub(1).min(ITEMS.len() - 1),
            KeyCode::Down | KeyCode::Tab => sel = (sel + 1) % ITEMS.len(),
            KeyCode::Enter => act(sel, &mut out),
            KeyCode::Char(c) => {
                let idx = match c {
                    '1'..='9' => (c as u8 - b'1') as usize,
                    '0' => 9,
                    _ => continue,
                };
                if idx < ITEMS.len() {
                    sel = idx;
                    act(sel, &mut out);
                }
            }
            _ => {}
        }
    }

    // restore terminal
    print!("\x1b[?25h\x1b[0m\r\n");
    out.flush().ok();
    println!(
        "  {DIM}price database travels with you — {RESET}{CYAN}costcrew spend add 19.99{RESET} {DIM}from any shell{RESET}"
    );
}

fn act(sel: usize, out: &mut std::io::Stdout) {
    print!("\x1b[2J\x1b[H\x1b[?25h");
    out.flush().ok();

    match sel {
        0 => {
            let q = prompt("filter the database (enter = everything)");
            let mut args = base(Cmd::Prices);
            if !q.is_empty() {
                args.pos.push(q);
            }
            crate::flow(args);
        }
        1 => crate::flow(base(Cmd::Spend)),
        2 => crate::spend::interactive_add(),
        3 => {
            let mut args = base(Cmd::Leaderboard);
            args.top = 25;
            crate::flow(args);
        }
        4 => crate::flow(base(Cmd::Catalog)),
        5 => {
            let key = match std::env::var("OPENROUTER_API_KEY") {
                Ok(k) => k,
                Err(_) => prompt("OpenRouter key (sk-or-v1-...)"),
            };
            let mut args = base(Cmd::OpenRouter);
            if !key.is_empty() {
                args.key = Some(key);
            }
            crate::flow(args);
        }
        6 => crate::flow(base(Cmd::Fetch)),
        7 => crate::flow(base(Cmd::Scan)),
        8 => crate::flow(base(Cmd::Advise)),
        9 => crate::flow(base(Cmd::Demo)),
        _ => {}
    }

    // "press any key" pause, then back to the menu
    println!();
    println!("  {DIM}press any key to go back{RESET}");
    std::io::stdout().flush().ok();
    let _ = wait_key();
}

fn base(cmd: Cmd) -> Args {
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

// ---------- splash ----------

fn splash(out: &mut std::io::Stdout) {
    println!();
    for (i, line) in BANNER.iter().enumerate() {
        println!("{}{line}{RESET}", GRAD[i % GRAD.len()]);
    }
    println!();
    println!(
        "  {BOLD}the AI coding price database{RESET} {DIM}+ your spend · v{}{RESET}",
        env!("CARGO_PKG_VERSION")
    );

    // little spinner while the DB loads — same vibe as the big CLIs
    let frames = ["-", "\\", "|", "/"];
    for f in frames.iter().cycle().take(14) {
        print!("\r  {CYAN}{f}{RESET} {DIM}loading bundled price database ...{RESET} ");
        out.flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(45));
    }
}

// ---------- menu ----------

fn draw_menu(out: &mut std::io::Stdout, db: &PriceDB, sel: usize) {
    print!("\x1b[2J\x1b[H\x1b[?25h");
    println!();
    println!(
        "  {BOLD}costcrew{RESET} {DIM}· {} models bundled · arrow keys or numbers{RESET}",
        db.entries().len()
    );
    println!();
    for (i, it) in ITEMS.iter().enumerate() {
        let num = if i == 9 {
            "0".to_string()
        } else {
            format!("{}", i + 1)
        };
        if i == sel {
            println!(
                "  {CYAN}▸ {num} {BOLD}{:<20}{RESET} {CYAN}{}{RESET}",
                it.name, it.hint
            );
        } else {
            println!(
                "    {DIM}{num} {:<20}{RESET} {DIM}{}{RESET}",
                it.name, it.hint
            );
        }
    }
    println!();
    println!("  {YELLOW}{FOOTER}{RESET}");
    out.flush().ok();
}

// ---------- terminal helpers ----------

/// one key press (skips release events); raw mode only while waiting
fn wait_key() -> Option<KeyEvent> {
    loop {
        if enable_raw_mode().is_err() {
            return None;
        }
        let ev = read().ok();
        let _ = disable_raw_mode();
        if let Some(Event::Key(k)) = ev {
            if k.kind == KeyEventKind::Press {
                return Some(k);
            }
        } else if ev.is_none() {
            return None;
        }
    }
}

fn prompt(label: &str) -> String {
    print!("  {CYAN}{label}{RESET} ");
    std::io::stdout().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

// no style-check shim needed — imports are exact
