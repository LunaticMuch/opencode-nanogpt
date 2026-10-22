use crate::cli::GlobalArgs;
use anyhow::Result;

/// Placeholder for future formatting of the nanogpt section only.
///
/// Deferred to v0.2 for two reasons:
///   1. The JSONC patcher already preserves user formatting exactly, so
///      "format" would only apply to the section this tool owns — and that
///      section is written by us, so it is already consistent.
///   2. Any formatter that rewrites the user's comments or whitespace
///      outside the nanogpt section would violate the tool's core
///      promise (surgical edits only). A real formatter must therefore
///      operate on the same patcher primitives, which makes it a natural
///      follow-up rather than a v0.1 concern.
///
/// For now: exit 0 with a clear message so scripts that call `format`
/// do not break, but the user knows nothing happened.
pub fn run(_global: GlobalArgs) -> Result<()> {
    eprintln!(
        "format: no-op in v0.1 — the nanogpt section is already written \
         consistently by init/update-models; a section-only formatter is \
         planned for a later release"
    );
    Ok(())
}
