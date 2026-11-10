//! `costcrew catalog` — what every coding agent can run on, and how it bills.

use crate::report::{BOLD, CYAN, DIM, RESET};

pub struct Agent {
    pub name: &'static str,
    /// how it bills
    pub billing: &'static str,
    /// can it run on arbitrary API keys?
    pub byok: &'static str,
    /// whether costcrew can read its logs
    pub scanned: &'static str,
    pub models: &'static str,
    pub url: &'static str,
}

const CATALOG: &[Agent] = &[
    Agent {
        name: "Claude Code",
        billing: "Pro/Max subscription (weekly caps) OR pay-per-token API",
        byok: "yes (Anthropic key)",
        scanned: "yes (JSONL transcripts)",
        models: "claude-opus / sonnet / haiku (all tiers)",
        url: "docs.anthropic.com/claude-code",
    },
    Agent {
        name: "OpenCode",
        billing: "free, open source; you pay the model",
        byok: "yes — Anthropic, OpenAI, Google, OpenRouter, GLM, local",
        scanned: "yes (sqlite + JSON)",
        models: "any provider; public models include big-pickle (free tier)",
        url: "opencode.ai",
    },
    Agent {
        name: "Codex CLI",
        billing: "bundled with ChatGPT Plus/Pro; API per-token otherwise",
        byok: "OpenAI key",
        scanned: "best-effort (JSONL)",
        models: "gpt-5 codex line (fast/slow lanes)",
        url: "github.com/openai/codex",
    },
    Agent {
        name: "Gemini CLI",
        billing: "free tier ~1,000 req/day with Google login; Vertex/Studio per-token",
        byok: "yes (GCP key)",
        scanned: "not yet",
        models: "gemini 3 / 2 nano",
        url: "github.com/google-gemini/gemini-cli",
    },
    Agent {
        name: "GitHub Copilot CLI",
        billing: "Copilot Pro/Pro+ subscription; some models bill per premium-request",
        byok: "no",
        scanned: "not yet",
        models: "gpt + claude + gemini, managed by GitHub policy",
        url: "github.com/features/copilot",
    },
    Agent {
        name: "Kilo Code",
        billing: "extension, free; pay via OpenRouter or your own API keys",
        byok: "yes (OpenRouter recommended)",
        scanned: "yes-best-effort (task JSON)",
        models: "anything on OpenRouter incl. free deepseek/qwen lanes",
        url: "kilocode.ai",
    },
    Agent {
        name: "Cline",
        billing: "extension, free; bring API keys (incl. Anthropic/OpenRouter)",
        byok: "yes — per-task cost shown in-app",
        scanned: "yes-best-effort (task JSON)",
        models: "any; plus auto-mode model picking",
        url: "cline.bot",
    },
    Agent {
        name: "Roo Code",
        billing: "extension, free; BYOK",
        byok: "yes; models scoped per mode (ask/code/architect)",
        scanned: "yes-best-effort (task JSON)",
        models: "anything through the major providers",
        url: "roocode.com",
    },
    Agent {
        name: "Aider",
        billing: "open source CLI; pay per API token; total printed per commit",
        byok: "yes — literally any openai-compatible endpoint incl. local Ollama",
        scanned: "yes (Cost lines in .aider.chat.history.md)",
        models: "any; pairs well with deepseek-chat",
        url: "aider.chat",
    },
    Agent {
        name: "Cursor",
        billing: "Pro subscription ($20) + usage-based add-ons for frontier models",
        byok: "limited",
        scanned: "not yet",
        models: "bundled: claude, gpt, gemini, cursor's own mini models",
        url: "cursor.com",
    },
    Agent {
        name: "Windsurf",
        billing: "sub plans with monthly credit pool; overage billed",
        byok: "yes on some tiers",
        scanned: "not yet",
        models: "bundled claude/gpt + their ide models",
        url: "windsurf.com",
    },
    Agent {
        name: "self-hosted (vLLM/ollama/llama.cpp)",
        billing: "GPU rental from $1–3/hr (cloud) to free (local); electricity",
        byok: "n/a — it IS the server",
        scanned: "not yet (roadmap: opencode sessions cover it)",
        models: "llama, qwen, deepseek, glm, mistral — open weights",
        url: "docs.vllm.ai",
    },
];

pub fn print_catalog() {
    println!();
    println!(
        "  {BOLD}costcrew catalog{RESET} {DIM}· who runs what · list-price snapshot mid-2026{RESET}"
    );
    println!();

    for a in CATALOG {
        println!("  {CYAN}{BOLD}{:<33}{RESET}", a.name);
        println!("  {DIM}billing :{RESET} {}", a.billing);
        println!("  {DIM}models  :{RESET} {}", a.models);
        println!("  {DIM}byok    :{RESET} {}", a.byok);
        println!("  {DIM}scanned :{RESET} {}", a.scanned);
        println!("  {DIM}link    :{RESET} {}", a.url);
        println!();
    }
    println!("  {DIM}got a lane we're missing? open an issue: github.com/Arch087146/costcrew{RESET}");
    println!();
}
