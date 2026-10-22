use serde_json::{json, Value};

/// Build the MCP server definition injected into `opencode.json`.
///
/// Package name verified against:
/// https://docs.nano-gpt.com/integrations/mcp
/// (Option 1: `npx -y @nanogpt/mcp`)[reference:0]
pub fn build_mcp_block(enabled: bool) -> Value {
    json!({
        "type": "local",
        "command": ["npx", "-y", "@nanogpt/mcp"],
        "enabled": enabled,
        "environment": { "NANOGPT_API_KEY": "{env:NANOGPT_API_KEY}" }
    })
}

pub const COST_WARNING: &str = "\
NanoGPT MCP written. When `enabled` is true, its tools (search, scrape, media)
bill your NanoGPT balance on each call and consume model context. See
https://docs.nano-gpt.com/integrations/mcp";
