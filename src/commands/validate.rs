use crate::cli::GlobalArgs;
use crate::config::jsonc::find_value_span;
use crate::paths;
use anyhow::{bail, Result};

/// Verify that the nanogpt section of `opencode.json` is well-formed.
///
/// Checks, in order:
///   - the config file exists and is readable
///   - `provider.nanogpt` is present
///   - `provider.nanogpt.options.baseURL` is present and is a string
///   - `provider.nanogpt.options.apiKey` is present and is a string
///   - `provider.nanogpt.models` is present and is an object
///   - if `mcp.nanogpt` is present, it declares a `command` array
///
/// Exit code:
///   0  all checks pass
///   2  one or more checks fail (an `anyhow::Error` carries the summary)
pub fn run(global: GlobalArgs) -> Result<()> {
    let path = paths::resolve(global.config.as_deref())?;
    if !path.exists() {
        bail!("config not found: {}", path.display());
    }
    let src = std::fs::read_to_string(&path)?;

    let mut ok = true;

    check_present(&src, &["provider", "nanogpt"], &mut ok);

    if let Some((s, e)) = check_present(
        &src,
        &["provider", "nanogpt", "options", "baseURL"],
        &mut ok,
    ) {
        check_is_string(&src[s..e], "baseURL", &mut ok);
    }

    if let Some((s, e)) = check_present(
        &src,
        &["provider", "nanogpt", "options", "apiKey"],
        &mut ok,
    ) {
        check_is_string(&src[s..e], "apiKey", &mut ok);
    }

    if let Some((s, e)) = check_present(
        &src,
        &["provider", "nanogpt", "models"],
        &mut ok,
    ) {
        check_is_object(&src[s..e], "models", &mut ok);
    }

    // Optional: the MCP block, if present, must have a `command` array.
    if find_value_span(src.as_bytes(), &["mcp", "nanogpt"])?.is_some() {
        check_present(&src, &["mcp", "nanogpt", "command"], &mut ok);
    }

    if !ok {
        bail!("nanogpt section is incomplete (see MISS lines above)");
    }
    eprintln!("all checks passed");
    Ok(())
}

/// Returns `Some((start, end))` on success; `None` and flips `ok` on miss.
fn check_present(src: &str, path: &[&str], ok: &mut bool) -> Option<(usize, usize)> {
    match find_value_span(src.as_bytes(), path) {
        Ok(Some(span)) => {
            eprintln!("OK   {}", path.join("."));
            Some(span)
        }
        Ok(None) => {
            eprintln!("MISS {}", path.join("."));
            *ok = false;
            None
        }
        Err(e) => {
            eprintln!("MISS {} ({e})", path.join("."));
            *ok = false;
            None
        }
    }
}

fn check_is_string(slice: &str, label: &str, ok: &mut bool) {
    if serde_json::from_str::<serde_json::Value>(slice)
        .map(|v| v.is_string())
        .unwrap_or(false)
    {
        eprintln!("OK   {label} is a string");
    } else {
        eprintln!("BAD  {label} is not a string");
        *ok = false;
    }
}

fn check_is_object(slice: &str, label: &str, ok: &mut bool) {
    // The scanner may have matched a span that includes trailing whitespace
    // or comments; parse it as JSON to be sure it is an object.
    if serde_json::from_str::<serde_json::Value>(slice)
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        eprintln!("OK   {label} is an object");
    } else {
        eprintln!("BAD  {label} is not an object");
        *ok = false;
    }
}
