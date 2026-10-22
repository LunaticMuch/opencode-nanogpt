use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Resolve the config path:
///   1. Explicit `--config` argument, if given
///   2. `$XDG_CONFIG_HOME/opencode/opencode.json`, if that file exists
///   3. `~/.config/opencode/opencode.json` (whether or not it exists)
pub fn resolve(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("opencode").join("opencode.json");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".config").join("opencode").join("opencode.json"));
    }
    Err(anyhow!("could not resolve config path"))
}
