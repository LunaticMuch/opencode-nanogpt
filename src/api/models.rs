use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModelPricing {
    #[serde(default)]
    pub prompt: Option<f64>,
    #[serde(default)]
    pub completion: Option<f64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "context_length")]
    pub context_length: Option<i64>,
    #[serde(default, rename = "max_output_tokens")]
    pub max_output_tokens: Option<i64>,
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
}

/// Tolerant parser for the NanoGPT `/models` response.
///
/// Accepts:
///   - `{ "model-id": { ... }, "other-id": { ... } }` (NanoGPT's map form)
///   - `{ "data": [ { "id": "...", "created": ... }, ... ] }` (OpenAI-style)
///   - `{ "models": [ ... ] }`
///   - `[ { "id": ... }, ... ]` (bare array)
///
/// For the map form, the top-level key is the model ID and any `created`
/// field found inside the value is used when present.
pub fn parse_models(v: &Value) -> Result<Vec<ModelEntry>> {
    // Case 1: bare array
    if let Some(arr) = v.as_array() {
        return Ok(from_array(arr));
    }

    // Case 2: object with `data` or `models` array
    if let Some(arr) = v.get("data").and_then(|d| d.as_array())
        .or_else(|| v.get("models").and_then(|d| d.as_array()))
    {
        return Ok(from_array(arr));
    }

    // Case 3: object keyed by model ID (NanoGPT's native shape)
    if let Some(obj) = v.as_object() {
        if obj.is_empty() {
            bail!("models response was an empty object");
        }
        let mut out = Vec::with_capacity(obj.len());
        for (id, val) in obj {
            out.push(entry_from(id.clone(), val));
        }
        return Ok(out);
    }

    bail!("unrecognized models response shape")
}

fn from_array(arr: &[Value]) -> Vec<ModelEntry> {
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
            out.push(entry_from(id.to_string(), item));
        }
    }
    out
}

fn entry_from(id: String, val: &Value) -> ModelEntry {
    let created = val.get("created").and_then(|x| x.as_i64());
    let name = val.get("name").and_then(|x| x.as_str()).map(str::to_string);
    let description = val.get("description").and_then(|x| x.as_str()).map(str::to_string);
    let context_length = val.get("context_length").and_then(|x| x.as_i64());
    let max_output_tokens = val.get("max_output_tokens").and_then(|x| x.as_i64());
    let pricing = val.get("pricing").cloned().and_then(|p| serde_json::from_value(p).ok());
    let capabilities = val.get("capabilities").cloned();
    ModelEntry {
        id,
        created,
        name,
        description,
        context_length,
        max_output_tokens,
        pricing,
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_map_form() {
        let v = json!({
            "deepseek/deepseek-v4-flash": { "price": 0.1 },
            "openai/gpt-4o": { "price": 0.5, "created": 300 }
        });
        let models = parse_models(&v).unwrap();
        assert_eq!(models.len(), 2);
        let gpt = models.iter().find(|m| m.id == "openai/gpt-4o").unwrap();
        assert_eq!(gpt.created, Some(300));
    }

    #[test]
    fn parses_data_array_form() {
        let v = json!({
            "data": [
                { "id": "a/one", "created": 1 },
                { "id": "a/two", "created": 2 }
            ]
        });
        let models = parse_models(&v).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].id, "a/two");
    }

    #[test]
    fn parses_bare_array() {
        let v = json!([{ "id": "x" }, { "id": "y" }]);
        let models = parse_models(&v).unwrap();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn empty_object_errors() {
        let v = json!({});
        assert!(parse_models(&v).is_err());
    }
}
