use super::family::family_key;
use crate::api::models::ModelEntry;

/// Sort key used to rank models within a family.
///
/// Ordering priority (highest first):
///   1. `primary` bucket: 2 = has API `created` timestamp,
///                        1 = has parseable version tokens,
///                        0 = fallback (nothing to sort on)
///   2. `timestamp` (when present)
///   3. `version` tokens, compared lexicographically as numbers
///   4. `length` of the original ID (longer, dated IDs rank newer)
///
/// The tie-break on `length` matches the intent of "a dated ID
/// (`-0731`) is newer than an undated base". If two IDs differ only
/// in length, the longer one wins — and any model whose ranking we
/// cannot determine simply falls to the bottom of its family without
/// being dropped.
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct RankKey {
    primary: u8,
    timestamp: i64,
    version: Vec<u64>,
    length: usize,
}

pub fn rank_key(m: &ModelEntry) -> RankKey {
    let version = parse_version_tokens(&m.id);
    let length = m.id.len();

    if let Some(ts) = m.created {
        RankKey {
            primary: 2,
            timestamp: ts,
            version,
            length,
        }
    } else if !version.is_empty() {
        RankKey {
            primary: 1,
            timestamp: 0,
            version,
            length,
        }
    } else {
        RankKey {
            primary: 0,
            timestamp: 0,
            version,
            length,
        }
    }
}

fn parse_version_tokens(id: &str) -> Vec<u64> {
    let base = family_key(id);
    let mut out = Vec::new();
    let mut num = String::new();
    for c in base.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if !num.is_empty() {
            if let Ok(n) = num.parse::<u64>() {
                out.push(n);
            }
            num.clear();
        }
    }
    if !num.is_empty() {
        if let Ok(n) = num.parse::<u64>() {
            out.push(n);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(id: &str, created: Option<i64>) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            created,
            name: None,
            description: None,
            context_length: None,
            max_output_tokens: None,
            pricing: None,
            capabilities: None,
            subscription: None,
        }
    }

    #[test]
    fn created_timestamp_wins() {
        let older = rank_key(&mk("deepseek/deepseek-v4-flash-0120", Some(100)));
        let newer = rank_key(&mk("deepseek/deepseek-v4-flash-0731", Some(200)));
        assert!(newer > older);
    }

    #[test]
    fn version_tokens_used_without_timestamp() {
        let older = rank_key(&mk("x/model-2", None));
        let newer = rank_key(&mk("x/model-4", None));
        assert!(newer > older);
    }

    #[test]
    fn dated_beats_undated_without_timestamp() {
        // Longer, dated IDs rank above undated bases when no timestamp.
        let undated = rank_key(&mk("deepseek-v4", None));
        let dated = rank_key(&mk("deepseek-v4-0731", None));
        // `deepseek-v4` has version tokens [4]; `deepseek-v4-0731` has [4, 731]
        // so `dated` ranks strictly higher on version tokens already.
        assert!(dated > undated);
    }

    #[test]
    fn fallback_models_sort_below_others() {
        let with_ts = rank_key(&mk("x/anything", Some(1)));
        let no_signal = rank_key(&mk("x/", None));
        assert!(with_ts > no_signal);
    }
}
