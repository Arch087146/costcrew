//! Per-model pricing. Approximate list prices, per 1M tokens, USD.
//! Update freely — prefix-matched, longest prefix wins.

#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    /// USD per 1M tokens
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

const TABLE: &[(&str, Pricing)] = &[
    // --- Anthropic ---
    (
        "claude-opus-4",
        Pricing {
            input: 15.0,
            output: 75.0,
            cache_read: 1.5,
            cache_write: 18.75,
        },
    ),
    (
        "claude-sonnet-4",
        Pricing {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
        },
    ),
    (
        "claude-haiku-4",
        Pricing {
            input: 1.0,
            output: 5.0,
            cache_read: 0.1,
            cache_write: 1.25,
        },
    ),
    (
        "claude-3-7-sonnet",
        Pricing {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
        },
    ),
    // --- OpenAI ---
    (
        "gpt-5",
        Pricing {
            input: 1.25,
            output: 10.0,
            cache_read: 0.125,
            cache_write: 1.25,
        },
    ),
    (
        "o3",
        Pricing {
            input: 2.0,
            output: 8.0,
            cache_read: 0.5,
            cache_write: 2.0,
        },
    ),
    (
        "gpt-4.1",
        Pricing {
            input: 2.0,
            output: 8.0,
            cache_read: 0.5,
            cache_write: 2.0,
        },
    ),
    (
        "codex",
        Pricing {
            input: 1.25,
            output: 10.0,
            cache_read: 0.125,
            cache_write: 1.25,
        },
    ),
    // --- Google ---
    (
        "gemini",
        Pricing {
            input: 1.25,
            output: 10.0,
            cache_read: 0.31,
            cache_write: 0.0,
        },
    ),
    // --- Zhipu ---
    (
        "glm-5",
        Pricing {
            input: 1.0,
            output: 3.2,
            cache_read: 0.11,
            cache_write: 1.0,
        },
    ),
    (
        "glm-4",
        Pricing {
            input: 0.6,
            output: 2.2,
            cache_read: 0.11,
            cache_write: 0.6,
        },
    ),
    // --- Others ---
    (
        "deepseek-chat",
        Pricing {
            input: 0.27,
            output: 1.1,
            cache_read: 0.07,
            cache_write: 0.27,
        },
    ),
    (
        "deepseek-reasoner",
        Pricing {
            input: 0.55,
            output: 2.19,
            cache_read: 0.14,
            cache_write: 0.55,
        },
    ),
    (
        "qwen",
        Pricing {
            input: 0.4,
            output: 1.2,
            cache_read: 0.04,
            cache_write: 0.4,
        },
    ),
];

const UNKNOWN: Pricing = Pricing {
    input: 0.0,
    output: 0.0,
    cache_read: 0.0,
    cache_write: 0.0,
};

/// Longest-prefix match so e.g. `claude-opus-4-5-20260101` hits `claude-opus-4`.
pub fn lookup(model: &str) -> Pricing {
    let mut best: Option<&str> = None;
    for (prefix, _) in TABLE {
        if model.to_lowercase().starts_with(prefix) {
            match best {
                Some(b) if b.len() >= prefix.len() => {}
                _ => best = Some(prefix),
            }
        }
    }
    match best {
        Some(p) => TABLE
            .iter()
            .find(|(k, _)| *k == p)
            .map(|(_, v)| *v)
            .unwrap(),
        None => UNKNOWN,
    }
}
