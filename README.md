# costcrew

**Find out what your AI coding agents *actually* cost you.**

Desktop dashboards hide it behind your subscription. costcrew reads the usage logs
already sitting on your machine, applies per-model prices, and prints the bill —
per model, in dollars, right in your terminal.

```text
  costcrew · AI agent spend · last 7d

  provider                    model  reqs  tokens in  tokens out   cache  cost (usd)
  --------  ------------------------  ----  ---------  ----------  -------  ----------
    claude  claude-opus-4-5-20260115     2      71000       16400  4540000     19.2825
    claude         claude-sonnet-4-5     3      55500       13200  2968000      2.4900
  opencode                     glm-5     3     114000       28700   695000      0.4024
     codex             gpt-5.2-codex     2      20000        4000   801000      0.3125
  opencode          mystery-coder-xl     1      30000        8000    10000           ?
  --------  --------  total             290500        70300   9014000         $22.49

    claude-opus-4-5-20260115 ######################## $19.28
           claude-sonnet-4-5 ### $2.49
                       glm-5 # $0.40
               gpt-5.2-codex # $0.31

  1 model(s) not priced yet — saw them, didn't bill them.
```

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

## Install

```sh
# from source (Rust 1.80+)
git clone https://github.com/YOUR_USERNAME/costcrew
cargo install --path costcrew
```

Prebuilt binaries land on the [Releases](https://github.com/YOUR_USERNAME/costcrew/releases)
page when a tag is pushed. No runtime dependencies — everything's read from disk.

## Usage

```sh
costcrew                 # scan your machine, default lookback 30 days
costcrew --since 7d      # this week's damage
costcrew md > spend.md   # shareable markdown table (great for standups or gists)
costcrew --json          # machine-readable — pipe into a cron, a webhook, whatever
costcrew demo            # sample report from synthetic data, no scanning needed
```

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
