/// Compute the "family" of a model ID by stripping the provider prefix and
/// any trailing date-like suffix (`-0731`, `-202407`, `-20240731`).
///
/// Family examples:
///   deepseek/deepseek-v4-flash            → deepseek-v4-flash
///   deepseek/deepseek-v4-flash-0731       → deepseek-v4-flash
///   openai/gpt-4o-20240731                → gpt-4o
///   gpt-4o                                → gpt-4o
///
/// Distinct variants do NOT collapse:
///   deepseek-v4        → deepseek-v4
///   deepseek-v4-flash  → deepseek-v4-flash
pub fn family_key(id: &str) -> String {
    let base = id.split_once('/').map(|(_, r)| r).unwrap_or(id);
    strip_date_suffix(base).to_string()
}

fn strip_date_suffix(s: &str) -> &str {
    // Try longest suffix first: 8 digits (YYYYMMDD), then 6 (YYYYMM), then 4 (MMDD).
    for len in [8, 6, 4] {
        if s.len() > len + 1 && s.as_bytes()[s.len() - len - 1] == b'-' {
            let tail = &s[s.len() - len..];
            if tail.bytes().all(|b| b.is_ascii_digit()) {
                return &s[..s.len() - len - 1];
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_provider_prefix() {
        assert_eq!(family_key("openai/gpt-4o"), "gpt-4o");
        assert_eq!(family_key("gpt-4o"), "gpt-4o");
    }

    #[test]
    fn strips_date_suffixes() {
        assert_eq!(family_key("deepseek/deepseek-v4-flash-0731"), "deepseek-v4-flash");
        assert_eq!(family_key("openai/gpt-4o-20240731"), "gpt-4o");
        assert_eq!(family_key("anthropic/claude-3-202407"), "claude-3");
    }

    #[test]
    fn keeps_short_numeric_suffixes() {
        // `-4` is not long enough to be treated as a date suffix.
        assert_eq!(family_key("gpt-4"), "gpt-4");
        assert_eq!(family_key("claude-3"), "claude-3");
    }

    #[test]
    fn does_not_collapse_distinct_variants() {
        assert_ne!(family_key("deepseek-v4"), family_key("deepseek-v4-flash"));
        assert_ne!(family_key("gpt-4o"), family_key("gpt-4o-mini"));
    }
}
