use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "opencode-nanogpt", version, about = "Configure OpenCode for NanoGPT")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone)]
pub struct GlobalArgs {
    /// Path to opencode.json (default: XDG, then ~/.config/opencode/opencode.json)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    /// Print the resulting file to stdout; do not write
    #[arg(long, global = true)]
    pub dry_run: bool,
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create or update the nanogpt provider + MCP block
    Init(InitArgs),
    /// Refresh model list from the NanoGPT API, filtered
    UpdateModels(UpdateModelsArgs),
    /// Check that opencode.json has a valid nanogpt section
    Validate,
    /// Restore the most recent backup
    Rollback,
    /// Placeholder for future nanogpt-section formatting
    Format,
}

#[derive(Args, Clone)]
pub struct InitArgs {
    /// Env var name for the API key
    #[arg(long, default_value = "NANOGPT_API_KEY")]
    pub api_key_env: String,
    /// Base URL for the NanoGPT API
    #[arg(long, default_value = "https://nano-gpt.com/api/v1")]
    pub base_url: String,
    /// Write MCP block with enabled: true
    #[arg(long, conflicts_with_all = ["no_mcp", "mcp_disabled"])]
    pub mcp: bool,
    /// Omit MCP block entirely
    #[arg(long, conflicts_with_all = ["mcp", "mcp_disabled"])]
    pub no_mcp: bool,
    /// Write MCP block with enabled: false (default)
    #[arg(long, conflicts_with_all = ["mcp", "no_mcp"])]
    pub mcp_disabled: bool,
    /// Overwrite existing nanogpt section without prompting
    #[arg(long)]
    pub force: bool,
}

impl InitArgs {
    pub fn mcp_mode(&self) -> McpMode {
        if self.mcp {
            McpMode::Enabled
        } else if self.no_mcp {
            McpMode::Omit
        } else {
            McpMode::Disabled
        }
    }
}

pub enum McpMode {
    Enabled,
    Disabled,
    Omit,
}

#[derive(Args, Clone)]
pub struct UpdateModelsArgs {
    /// Keep the latest N per family
    #[arg(long, default_value_t = 2)]
    pub per_family: usize,
    /// Only keep models matching any include glob (repeatable)
    #[arg(long)]
    pub include: Vec<String>,
    /// Drop models matching any exclude glob (repeatable)
    #[arg(long)]
    pub exclude: Vec<String>,
    /// Remove models no longer in the API response (default: keep existing)
    #[arg(long)]
    pub prune: bool,
}
