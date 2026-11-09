//! Synthetic data so users can see the report before they've used any agent.

use crate::scan::{aggregate, Row, UsageEvent};

/// Roughly realistic fake week of usage.
pub fn demo_events() -> Vec<UsageEvent> {
    fn ev(provider: &str, model: &str, i: u64, o: u64, cr: u64, cw: u64) -> UsageEvent {
        UsageEvent {
            provider: provider.into(),
            model: model.into(),
            input_tokens: i,
            output_tokens: o,
            cache_read: cr,
            cache_write: cw,
            source: "demo".into(),
        }
    }

    vec![
        // a heavy week of Claude Code refactoring
        ev(
            "claude",
            "claude-opus-4-5-20260115",
            40_000,
            9_000,
            2_100_000,
            310_000,
        ),
        ev(
            "claude",
            "claude-opus-4-5-20260115",
            31_000,
            7_400,
            1_850_000,
            280_000,
        ),
        ev(
            "claude",
            "claude-sonnet-4-5",
            22_000,
            5_100,
            980_000,
            140_000,
        ),
        ev(
            "claude",
            "claude-sonnet-4-5",
            18_500,
            4_300,
            870_000,
            120_000,
        ),
        ev(
            "claude",
            "claude-sonnet-4-5",
            15_000,
            3_800,
            760_000,
            98_000,
        ),
        // OpenCode doing feature work on GLM
        ev("opencode", "glm-5", 48_000, 12_000, 240_000, 60_000),
        ev("opencode", "glm-5", 39_000, 9_800, 190_000, 44_000),
        ev("opencode", "glm-5", 27_000, 6_900, 130_000, 31_000),
        // Codex quick sessions
        ev("codex", "gpt-5.2-codex", 12_000, 2_400, 410_000, 80_000),
        ev("codex", "gpt-5.2-codex", 8_000, 1_600, 260_000, 51_000),
        // one model we don't have a price for
        ev("opencode", "mystery-coder-xl", 30_000, 8_000, 0, 10_000),
    ]
}

pub fn demo_rows() -> Vec<Row> {
    aggregate(&demo_events())
}
