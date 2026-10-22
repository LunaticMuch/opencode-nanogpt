use crate::backup;
use crate::cli::{GlobalArgs, InitArgs, McpMode};
use crate::commands::write_atomic;
use crate::config::jsonc::{find_value_span, upsert};
use crate::{mcp, paths};
use anyhow::Result;
use serde_json::json;
use std::io::{self, IsTerminal, Write};

pub fn run(global: GlobalArgs, args: InitArgs) -> Result<()> {
    let path = paths::resolve(global.config.as_deref())?;
    let original = std::fs::read_to_string(&path).unwrap_or_else(|_| "{\n}\n".to_string());

    // Guard: if the nanogpt provider already exists, require --force or
    // an interactive confirmation.
    if !args.force
        && !global.dry_run
        && find_value_span(original.as_bytes(), &["provider", "nanogpt"])?.is_some()
        && !confirm("provider.nanogpt already exists. Overwrite?")?
    {
        eprintln!("aborted");
        return Ok(());
    }

    // Allow the API key to be stored either as an env reference
    // (`{env:NAME}`) or as a literal string, depending on what the
    // user passed.
    let api_key_value: serde_json::Value = match &args.api_key {
        Some(key) => serde_json::Value::String(key.clone()),
        None => serde_json::Value::String(format!("{{env:{}}}", args.api_key_env)),
    };
    let provider = json!({
        "npm": "@ai-sdk/openai-compatible",
        "name": "NanoGPT",
        "options": {
            "baseURL": args.base_url,
            "apiKey": api_key_value,
        },
        "models": {}
    });

    let mut out = upsert(&original, &["provider", "nanogpt"], &provider)?;

    match args.mcp_mode() {
        McpMode::Omit => {}
        McpMode::Disabled | McpMode::Enabled => {
            let enabled = matches!(args.mcp_mode(), McpMode::Enabled);
            out = upsert(
                &out,
                &["mcp", "nanogpt"],
                &mcp::build_mcp_block(enabled, args.api_key.as_deref(), &args.api_key_env),
            )?;
            eprintln!("{}", mcp::COST_WARNING);
        }
    }

    if global.dry_run {
        print!("{out}");
    } else {
        backup::before_write(&path)?;
        write_atomic(&path, &out)?;
        eprintln!("wrote {}", path.display());
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    if !io::stdin().is_terminal() {
        return Ok(false);
    }
    eprint!("{prompt} [y/N] ");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}
