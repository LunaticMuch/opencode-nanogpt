use crate::api::client::fetch_models;
use crate::backup;
use crate::cli::{GlobalArgs, UpdateModelsArgs};
use crate::commands::write_atomic;
use crate::config::jsonc::{find_value_span, upsert};
use crate::filter::select::{select, Options};
use crate::paths;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub fn run(global: GlobalArgs, args: UpdateModelsArgs) -> Result<()> {
    let path = paths::resolve(global.config.as_deref())?;
    let original = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;

    // Pull the base URL from the existing config so we query the same
    // endpoint the user already pointed OpenCode at. Fall back to the
    // default only if the config does not yet contain one.
    let base_url = extract_base_url(&original)
        .unwrap_or_else(|| "https://nano-gpt.com/api/v1".into());

    // API key is optional here: `/models` works unauthenticated, but if
    // the config stores one (either as `{env:NAME}` or a literal string)
    // we send it anyway so the request is attributable and works against
    // any stricter deployments. Fall back to NANOGPT_API_KEY.
    let api_key = extract_api_key(&original)
        .or_else(|| std::env::var("NANOGPT_API_KEY").ok());

    eprintln!("fetching models from {base_url} …");
    let models = fetch_models(&base_url, api_key.as_deref(), true)?;
    eprintln!("got {} models", models.len());

    let ids = select(
        &models,
        &Options {
            per_family: args.per_family,
            include: &args.include,
            exclude: &args.exclude,
        },
    );
    eprintln!("retained {} after filtering", ids.len());

    if ids.is_empty() {
        anyhow::bail!(
            "selection produced 0 models — check --include / --exclude patterns"
        );
    }

    // Build the new `models` object. Each entry is a minimal descriptor
    // that OpenCode accepts; expand here if the schema ever needs more.
    let mut map = serde_json::Map::new();
    for id in &ids {
        let entry = models.iter().find(|m| m.id == *id);
        let mut obj = serde_json::Map::new();
        let name = entry
            .and_then(|e| e.name.clone())
            .unwrap_or_else(|| display_name(id));
        obj.insert("name".to_string(), serde_json::json!(name));

        if let Some(e) = entry {
            // Context/output limits, when the API provides them.
            if let Some(ctx) = e.context_length {
                let mut limit = serde_json::Map::new();
                limit.insert("context".to_string(), serde_json::json!(ctx));
                if let Some(out) = e.max_output_tokens {
                    limit.insert("output".to_string(), serde_json::json!(out));
                }
                if limit.contains_key("output") {
                    obj.insert("limit".to_string(), serde_json::Value::Object(limit));
                }
            }
            // Cost info (NanoGPT reports per-million-token prices).
            if let Some(pricing) = &e.pricing {
                if let (Some(input), Some(output)) = (pricing.prompt, pricing.completion) {
                    let mut cost = serde_json::Map::new();
                    cost.insert("input".to_string(), serde_json::json!(input));
                    cost.insert("output".to_string(), serde_json::json!(output));
                    obj.insert("cost".to_string(), serde_json::Value::Object(cost));
                }
            }
        }

        map.insert(id.clone(), serde_json::Value::Object(obj));
    }

    // Unless --prune is given, preserve any models already in the config
    // that are not in the API response. This protects user-added or
    // temporarily-unlisted models from being dropped by an update.
    if !args.prune {
        for id in existing_model_ids(&original) {
            if !map.contains_key(&id) {
                map.insert(id.clone(), json!({ "name": display_name(&id) }));
            }
        }
    }

    let value = Value::Object(map);
    let out = upsert(
        &original,
        &["provider", "nanogpt", "models"],
        &value,
    )?;

    if global.dry_run {
        print!("{out}");
    } else {
        backup::before_write(&path)?;
        write_atomic(&path, &out)?;
        eprintln!("wrote {}", path.display());
    }
    Ok(())
}

/// Human-readable display name derived from the model ID. Replace `-`/`_`
/// with spaces and drop the provider prefix so the OpenCode model picker
/// shows something legible.
fn display_name(id: &str) -> String {
    id.split('/')
        .next_back()
        .unwrap_or(id)
        .replace(['-', '_'], " ")
}

/// Read the current `provider.nanogpt.models` keys, if present.
fn existing_model_ids(src: &str) -> Vec<String> {
    let span = match find_value_span(
        src.as_bytes(),
        &["provider", "nanogpt", "models"],
    ) {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };
    let (s, e) = span;
    let slice = &src[s..e];
    match serde_json::from_str::<Value>(slice) {
        Ok(Value::Object(obj)) => obj.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// Read `provider.nanogpt.options.baseURL` from the config, if present.
fn extract_base_url(src: &str) -> Option<String> {
    let span = find_value_span(
        src.as_bytes(),
        &["provider", "nanogpt", "options", "baseURL"],
    )
    .ok()??;
    let (s, e) = span;
    serde_json::from_str::<String>(&src[s..e]).ok()
}

/// Read `provider.nanogpt.options.apiKey` from the config, if present.
/// Resolves `{env:NAME}` references to the corresponding environment
/// variable; literal strings are returned as-is.
fn extract_api_key(src: &str) -> Option<String> {
    let span = find_value_span(
        src.as_bytes(),
        &["provider", "nanogpt", "options", "apiKey"],
    )
    .ok()??;
    let (s, e) = span;
    let value: String = serde_json::from_str(&src[s..e]).ok()?;
    if let Some(name) = value
        .strip_prefix("{env:")
        .and_then(|v| v.strip_suffix('}'))
    {
        std::env::var(name).ok()
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_api_key_literal() {
        let src = r#"{ "provider": { "nanogpt": { "options": { "apiKey": "sk-literal" } } } }"#;
        assert_eq!(extract_api_key(src).as_deref(), Some("sk-literal"));
    }

    #[test]
    fn extract_api_key_env_ref_resolves() {
        std::env::set_var("NANOGPT_TEST_KEY", "resolved-value");
        let src = r#"{ "provider": { "nanogpt": { "options": { "apiKey": "{env:NANOGPT_TEST_KEY}" } } } }"#;
        assert_eq!(extract_api_key(src).as_deref(), Some("resolved-value"));
        std::env::remove_var("NANOGPT_TEST_KEY");
    }

    #[test]
    fn extract_api_key_missing_env_returns_none() {
        std::env::remove_var("NANOGPT_MISSING_KEY");
        let src = r#"{ "provider": { "nanogpt": { "options": { "apiKey": "{env:NANOGPT_MISSING_KEY}" } } } }"#;
        assert!(extract_api_key(src).is_none());
    }

    #[test]
    fn extract_api_key_absent() {
        let src = r#"{ "provider": { "nanogpt": { "options": {} } } }"#;
        assert!(extract_api_key(src).is_none());
    }
}
