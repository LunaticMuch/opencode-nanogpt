use super::models::{parse_models, ModelEntry};
use anyhow::{Context, Result};

/// Fetch available models from the NanoGPT API.
///
/// The endpoint is `GET {base_url}/models`. Auth uses the `x-api-key`
/// header, per NanoGPT's docs. Some deployments also accept
/// `Authorization: Bearer`; we send `x-api-key` because it is the
/// documented form and avoids clashing with strict proxies.
pub fn fetch_models(
    base_url: &str,
    api_key: Option<&str>,
    detailed: bool,
) -> Result<Vec<ModelEntry>> {
    let mut url = format!("{}/models", base_url.trim_end_matches('/'));
    if detailed {
        url.push_str("?detailed=true");
    }

    let mut req = ureq::get(&url).timeout(std::time::Duration::from_secs(20));
    if let Some(k) = api_key {
        req = req.set("x-api-key", k);
    }

    let resp = req.call().with_context(|| format!("GET {url}"))?;
    let val: serde_json::Value = resp.into_json().context("parsing response body")?;
    parse_models(&val)
}
