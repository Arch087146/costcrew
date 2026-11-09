# costcrew

**Find out what your AI coding agents *actually* cost you.**

Desktop dashboards hide it behind your subscription. costcrew reads the usage logs
already sitting on your machine, applies per-model prices, and prints the bill —
per model, in dollars, right in your terminal.

<p align="center">
  <img src="assets/demo.png" alt="costcrew demo — a table of AI agent spend per model with a bar chart" width="700">
</p>

That "mystery-coder-xl" row is deliberate: when costcrew doesn't recognize a model,
it shows what it found but refuses to invent a price.

## What it reads

| Tool | Source | Status |
|---|---|---|
| OpenCode | `opencode.db` (SQLite message table, legacy JSON storage) | ✅ |
| Claude Code | `~/.claude/**/*.jsonl` transcripts | ✅ |
| Codex CLI | `~/.codex/sessions/**/*.jsonl` | ✅ best-effort |

Everything is read **read-only** — the SQLite connection is opened `READ_ONLY`, and
no log file is ever touched. costcrew only ever reads files newer than `--since`.

Here it is five minutes after `cargo install`, reading a real OpenCode database —
note it found 325 requests for a model it has no price for and *scaled the unknown
model honestly* instead of making a number up:

<p align="center">
  <img src="assets/scan.png" alt="costcrew scanning a real OpenCode database — 325 requests, unpriced model shown with a question mark" width="620">
</p>

## Install

```sh
# from source (Rust 1.80+)
git clone https://github.com/Arch087146/costcrew
cargo install --path costcrew
```

Prebuilt binaries land on the [Releases](https://github.com/Arch087146/costcrew/releases)
page when a tag is pushed. No runtime dependencies — everything's read from disk.

## Usage

```sh
costcrew                 # scan your machine, default lookback 30 days
costcrew --since 7d      # this week's damage
costcrew md > spend.md   # shareable markdown table (great for standups or gists)
costcrew --json          # machine-readable — pipe into a cron, a webhook, whatever
costcrew demo            # sample report from synthetic data, no scanning needed
```

And `--help`, for people who judge CLIs by their help text:

<p align="center">
  <img src="assets/help.png" alt="costcrew --help output" width="580">
</p>

## Pricing

Prices live in [`src/price.rs`](src/price.rs) — a plain prefix-match table, longest
prefix wins, so `claude-opus-4-5-20260115` matches `claude-opus-4`. Costs are
approximate list prices (no discounts, no pro-plan pooling) and drift — treat the
totals as ≥85% accurate.

Missing your model? Add one line and PR it. Cache reads/writes are priced
separately where the API exposes them, otherwise the bucket is collapsed.

## Why a CLI and not a dashboard?

Because the dashboard already exists — inside each vendor's billing page — and
it'll tell you nothing about the model you ran through a different tool yesterday.
costcrew is one screen, one binary, and it works the same whether the spend came
from Claude Code, OpenCode, or whatever ships next week.

It's open source too, so "how much did I spend on agents last month" and
"show me the math" live in the same place.

## Contributing

Spotted a tool whose logs we don't parse? The scanner is intentionally dumb —
`src/scan.rs` walks a handful of roots, parses any JSON/JSONL that smells like
`modelID` + token counts, and sqLite for the rest.

- `cargo test` — unit tests for aggregation and pricing
- `cargo clippy`-clean; fmt-checked in CI

## License

MIT — see [LICENSE](LICENSE).
