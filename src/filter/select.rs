use super::{family::family_key, glob::matches, rank::rank_key};
use crate::api::models::ModelEntry;
use std::collections::HashMap;

pub struct Options<'a> {
    pub per_family: usize,
    pub include: &'a [String],
    pub exclude: &'a [String],
}

/// Apply include/exclude filters, group by family, keep the top N per
/// family by rank, and return the retained IDs in original API order.
///
/// Invariants:
///   - Every model ID in the output exists in the input.
///   - No ID is returned twice.
///   - Order matches the input's order (stable, predictable for diffs).
pub fn select(models: &[ModelEntry], opts: &Options) -> Vec<String> {
    let filtered: Vec<&ModelEntry> = models
        .iter()
        .filter(|m| {
            opts.include.is_empty() || opts.include.iter().any(|p| matches(p, &m.id))
        })
        .filter(|m| !opts.exclude.iter().any(|p| matches(p, &m.id)))
        .collect();

    let mut groups: HashMap<String, Vec<&ModelEntry>> = HashMap::new();
    for m in &filtered {
        groups.entry(family_key(&m.id)).or_default().push(m);
    }

    let mut chosen: Vec<String> = Vec::new();
    for (_, mut group) in groups {
        group.sort_by(|a, b| rank_key(b).cmp(&rank_key(a)));
        for m in group.iter().take(opts.per_family) {
            chosen.push(m.id.clone());
        }
    }

    // Preserve API order in the final list.
    let order: HashMap<&str, usize> = models
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    chosen.sort_by_key(|id| order.get(id.as_str()).copied().unwrap_or(usize::MAX));
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(id: &str, created: Option<i64>) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            created,
        }
    }

    #[test]
    fn latest_two_per_family() {
        let models = vec![
            mk("deepseek/deepseek-v4-flash-0731", Some(200)),
            mk("deepseek/deepseek-v4-flash-0120", Some(100)),
            mk("deepseek/deepseek-v4-flash-0101", Some(10)),
            mk("openai/gpt-4o", Some(300)),
            mk("openai/gpt-4o-mini", Some(400)),
        ];
        let opts = Options {
            per_family: 2,
            include: &[],
            exclude: &[],
        };
        let out = select(&models, &opts);

        assert!(out.contains(&"deepseek/deepseek-v4-flash-0731".to_string()));
        assert!(out.contains(&"deepseek/deepseek-v4-flash-0120".to_string()));
        assert!(!out.contains(&"deepseek/deepseek-v4-flash-0101".to_string()));
        assert!(out.contains(&"openai/gpt-4o".to_string()));
        assert!(out.contains(&"openai/gpt-4o-mini".to_string()));
    }

    #[test]
    fn include_and_exclude_combine() {
        let models = vec![
            mk("deepseek/deepseek-v4", None),
            mk("deepseek/deepseek-v4-free", None),
            mk("openai/gpt-4o", None),
        ];
        let opts = Options {
            per_family: 10,
            include: &["deepseek*".into()],
            exclude: &["*-free".into()],
        };
        let out = select(&models, &opts);
        assert_eq!(out, vec!["deepseek/deepseek-v4".to_string()]);
    }

    #[test]
    fn output_preserves_api_order() {
        let models = vec![
            mk("z/zeta", Some(10)),
            mk("a/alpha", Some(20)),
            mk("m/mu", Some(30)),
        ];
        let opts = Options {
            per_family: 1,
            include: &[],
            exclude: &[],
        };
        let out = select(&models, &opts);
        assert_eq!(
            out,
            vec![
                "z/zeta".to_string(),
                "a/alpha".to_string(),
                "m/mu".to_string()
            ]
        );
    }

    #[test]
    fn no_duplicates_in_output() {
        let models = vec![
            mk("x/model", Some(1)),
            mk("x/model-0731", Some(2)),
        ];
        let opts = Options {
            per_family: 5,
            include: &[],
            exclude: &[],
        };
        let out = select(&models, &opts);
        let mut sorted = out.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(out.len(), sorted.len());
    }
}
