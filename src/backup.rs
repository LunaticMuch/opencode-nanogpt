use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const KEEP: usize = 5;

/// Copy the current file to `<name>.bak.<timestamp>` before an in-place
/// write. Keeps the KEEP most recent backups. No-op if the file does not
/// exist yet.
pub fn before_write(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let ts = chrono::Local::now().format("%Y%m%dT%H%M%S");
    let backup = path.with_extension(format!("json.bak.{}", ts));
    fs::copy(path, &backup)
        .with_context(|| format!("backup {}", path.display()))?;
    prune(path)?;
    Ok(())
}

fn prune(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let prefix = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("opencode.json");

    let mut backups: Vec<_> = fs::read_dir(parent)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(prefix) && n.contains(".bak."))
                .unwrap_or(false)
        })
        .collect();
    backups.sort_by_key(|e| e.file_name());

    if backups.len() > KEEP {
        for e in &backups[..backups.len() - KEEP] {
            let _ = fs::remove_file(e.path());
        }
    }
    Ok(())
}
