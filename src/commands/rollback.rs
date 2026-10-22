use crate::backup;
use crate::cli::GlobalArgs;
use crate::commands::write_atomic;
use crate::paths;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Restore the most recent backup of the config file.
///
/// Behavior:
///   - Locates `opencode.json.bak.<timestamp>` files in the same directory
///     as the config, picks the newest, and restores it.
///   - The current config (if present) is copied aside as a new backup
///     first, so a mistaken rollback can itself be undone.
///   - `--dry-run` reports which backup would be restored without touching
///     the filesystem.
pub fn run(global: GlobalArgs) -> Result<()> {
    let path = paths::resolve(global.config.as_deref())?;
    let backups = list_backups(&path)?;
    let latest: &PathBuf = match backups.last() {
        Some(p) => p,
        None => bail!("no backups found for {}", path.display()),
    };

    if global.dry_run {
        eprintln!("would restore {} → {}", latest.display(), path.display());
        return Ok(());
    }

    // Preserve the current state as a backup before overwriting it. If the
    // file does not exist, there is nothing to preserve and this is a no-op.
    backup::before_write(&path)?;

    let content = std::fs::read_to_string(latest)?;
    write_atomic(&path, &content)?;
    eprintln!("restored {} → {}", latest.display(), path.display());
    Ok(())
}

fn list_backups(path: &Path) -> Result<Vec<PathBuf>> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let prefix = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("opencode.json");

    let mut found: Vec<PathBuf> = std::fs::read_dir(parent)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(prefix) && n.contains(".bak."))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    // Filenames are `opencode.json.bak.YYYYMMDDTHHMMSS`, so lexicographic
    // order matches chronological order.
    found.sort();
    Ok(found)
}
