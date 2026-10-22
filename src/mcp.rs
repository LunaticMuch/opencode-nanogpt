use serde_json::{json, Value};

/// Build the MCP server definition injected into `opencode.json`.
///
/// Package name verified against:
/// https://docs.nano-gpt.com/integrations/mcp
/// (Option 1: `npx -y @nanogpt/mcp`)[reference:0]
pub fn build_mcp_block(enabled: bool, api_key_static: Option<&str>, api_key_env: &str) -> Value {
    let env_value = match api_key_static {
        Some(key) => key.to_string(),
        None => format!("{{env:{api_key_env}}}"),
    };
    json!({
        "type": "local",
        "command": ["npx", "-y", "@nanogpt/mcp"],
        "enabled": enabled,
        "environment": { "NANOGPT_API_KEY": env_value }
    })
}

pub const COST_WARNING: &str = "\
NanoGPT MCP written. When `enabled` is true, its tools (search, scrape, media)
bill your NanoGPT balance on each call and consume model context. See
https://docs.nano-gpt.com/integrations/mcp";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn env_mode_writes_env_ref() {
        let v = build_mcp_block(true, None, "FOO_KEY");
        assert_eq!(v["environment"]["NANOGPT_API_KEY"], "{env:FOO_KEY}");
        assert_eq!(v["enabled"], true);
    }

    #[test]
    fn static_mode_writes_literal() {
        let v = build_mcp_block(false, Some("sk-secret"), "NANOGPT_API_KEY");
        assert_eq!(v["environment"]["NANOGPT_API_KEY"], "sk-secret");
        assert_eq!(v["enabled"], false);
    }
}
