pub mod format;
pub mod init;
pub mod select;
pub mod rollback;
pub mod update_models;
pub mod validate;

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Write `content` to `path` atomically: write to a temp file in the same
/// directory, fsync, then rename over the target. On success the target
/// is either the old file or the new one — never a partially-written mix.
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("path has no parent directory")?;
    std::fs::create_dir_all(parent)?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(content.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    Ok(())
}
