//! `costcrew` interactive mode — the fancy bit.
//!
//! Bare `costcrew` opens this instead of dumping a table: the banner builds
//! itself in, a spinner runs while the bundled database loads, menu items
//! cascade in — then it's arrow keys. When you pick an action the logo
//! clears away and the real output takes the screen; any key returns you to
//! the menu.
//!
//! Rendering is plain ANSI (same style constants as the rest of the app);
//! crossterm is used only to capture key presses. Anti-flash notes:
//!   - raw mode is enabled ONCE for the whole menu loop (toggling it per
//!     keypress is what caused the flicker), and only dropped while a view
//!     runs with normal `println!` output;
//!   - every menu frame is composed into one String and written in a single
//!     `write_all` — partial repaints are what read as "flashing";
//!   - frames use `\r\n` line endings, required while raw mode is on.

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

/// "COST / CREW" in solid blocks (double-line box glyphs render as tofu in
/// plain conhost; solid █ renders everywhere)
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

const CLEAR: &str = "\x1b[2J\x1b[H\x1b[?25l";

pub fn run_app() {
    let mut out = std::io::stdout();
    let _ = out.write_all(CLEAR.as_bytes());

    // --- animated build-up: banner cascades, subtitle, spinner, items ---
    for (i, line) in BANNER.iter().enumerate() {
        write_line(&mut out, &format!("{}{line}{RESET}", GRAD[i % GRAD.len()]));
        std::thread::sleep(std::time::Duration::from_millis(55));
    }
    write_line(
        &mut out,
        &format!(
            "\r\n  {BOLD}the AI coding price database{RESET} {DIM}+ your spend · v{}{RESET}",
            env!("CARGO_PKG_VERSION")
        ),
    );

    // load the DB (it's fast — the spinner is for the vibe, like the big CLIs)
    let frames = ["-", "\\", "|", "/"];
    let db = {
        let mut db = PriceDB::builtin();
        let mut f = frames.iter().cycle();
        let mut loaded = false;
        for _ in 0..24 {
            if !loaded {
                crate::fetch::cached_into_db(&mut db);
                loaded = true;
            }
            let n = db.entries().len();
            write_inline(
                &mut out,
                &format!("\r  {CYAN}{}{RESET} {DIM}loading bundled price database · {n} models ...{RESET}  ", f.next().unwrap()),
            );
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        write_inline(&mut out, "\r\x1b[2K"); // wipe the spinner line
        db
    };

    // --- menu items cascade in ---
    let mut sel = 0usize;
    draw_menu(&mut out, &db, sel, true);

    // --- input loop: raw mode stays ON so keypresses don't flicker ---
    if enable_raw_mode().is_err() {
        return;
    }
    while let Some(key) = read_key() {
        match key.code {
            KeyCode::Esc => break,
            KeyCode::Char('q') | KeyCode::Char('Q') => break,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Up => {
                sel = sel.wrapping_sub(1).min(ITEMS.len() - 1);
                draw_menu(&mut out, &db, sel, false);
            }
            KeyCode::Down | KeyCode::Tab => {
                sel = (sel + 1) % ITEMS.len();
                draw_menu(&mut out, &db, sel, false);
            }
            KeyCode::Enter => {
                let _ = disable_raw_mode(); // views print with plain println!
                act(sel, &mut out);
                if enable_raw_mode().is_err() {
                    break;
                }
                draw_menu(&mut out, &db, sel, false);
            }
            KeyCode::Char(c) => {
                let idx = match c {
                    '1'..='9' => (c as u8 - b'1') as usize,
                    '0' => 9,
                    _ => continue,
                };
                if idx < ITEMS.len() {
                    sel = idx;
                    let _ = disable_raw_mode();
                    act(sel, &mut out);
                    if enable_raw_mode().is_err() {
                        break;
                    }
                    draw_menu(&mut out, &db, sel, false);
                }
            }
            _ => {}
        }
    }

    // restore terminal
    let _ = disable_raw_mode();
    let _ = out.write_all(b"\x1b[?25h\x1b[0m\r\n");
    out.flush().ok();
    println!(
        "  {DIM}price database travels with you — {RESET}{CYAN}costcrew spend add 19.99{RESET} {DIM}from any shell{RESET}"
    );
}

/// run one menu item: clear the logo away, do the thing, wait for a key
fn act(sel: usize, out: &mut std::io::Stdout) {
    let _ = out.write_all("\x1b[2J\x1b[H\x1b[?25h".as_bytes());
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
    let _ = wait_any_key();
}

// ---------- frames ----------

/// full menu frame — banner on top of the options, always
fn frame(db: &PriceDB, sel: usize) -> String {
    let mut s = String::from(CLEAR);
    for (i, line) in BANNER.iter().enumerate() {
        s.push_str(GRAD[i % GRAD.len()]);
        s.push_str(line);
        s.push_str(RESET);
        s.push_str("\r\n");
    }
    s.push_str(&format!(
        "\r\n  {BOLD}costcrew{RESET} {DIM}· {} models bundled · offline{RESET}\r\n\r\n",
        db.entries().len()
    ));
    for (i, it) in ITEMS.iter().enumerate() {
        let num = if i == 9 {
            "0".to_string()
        } else {
            format!("{}", i + 1)
        };
        if i == sel {
            s.push_str(&format!(
                "  {CYAN}▸ {num} {BOLD}{:<20}{RESET} {CYAN}{}{RESET}\r\n",
                it.name, it.hint
            ));
        } else {
            s.push_str(&format!(
                "    {DIM}{num} {:<20}{RESET} {DIM}{}{RESET}\r\n",
                it.name, it.hint
            ));
        }
    }
    s.push_str(&format!("\r\n  {YELLOW}{FOOTER}{RESET}\r\n"));
    s
}

fn draw_menu(out: &mut std::io::Stdout, db: &PriceDB, sel: usize, animate: bool) {
    let f = frame(db, sel);
    if animate {
        for line in f.split("\r\n") {
            write_line(out, line);
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    } else {
        // single atomic write — no partial repaint, no flash
        let _ = out.write_all(f.as_bytes());
        out.flush().ok();
    }
}

fn write_line(out: &mut std::io::Stdout, s: &str) {
    let _ = out.write_all(s.as_bytes());
    let _ = out.write_all(b"\r\n");
    out.flush().ok();
}

fn write_inline(out: &mut std::io::Stdout, s: &str) {
    let _ = out.write_all(s.as_bytes());
    out.flush().ok();
}

// ---------- terminal helpers ----------

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

/// one key press (skips release events); requires raw mode to be ON
fn read_key() -> Option<KeyEvent> {
    loop {
        match read() {
            Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => return Some(k),
            Ok(_) => continue, // release events, resize, focus, etc.
            Err(_) => return None,
        }
    }
}

/// for the "press any key" pause — raw mode is OFF here, toggles it itself
fn wait_any_key() -> Option<KeyEvent> {
    if enable_raw_mode().is_err() {
        return None;
    }
    let k = read_key();
    let _ = disable_raw_mode();
    k
}

fn prompt(label: &str) -> String {
    print!("  {CYAN}{label}{RESET} ");
    std::io::stdout().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}
