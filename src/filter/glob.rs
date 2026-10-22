/// Minimal `*`-only glob matcher, sufficient for CLI `--include` /
/// `--exclude` patterns.
///
/// Semantics:
///   - `*` matches any run of characters (including empty)
///   - everything else is literal
///   - patterns without `*` are exact-match tests
///
/// This is deliberately not a full glob implementation. It exists so the
/// agent (and future maintainers) never have to pull in a glob crate for
/// three predicates; the algorithm is small enough to audit at a glance.
pub fn matches(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();

    // Anchor at start unless pattern begins with `*`.
    if !parts[0].is_empty() && !text.starts_with(parts[0]) {
        return false;
    }

    // Anchor at end unless pattern ends with `*`.
    let last = parts.last().copied().unwrap_or("");
    if !pattern.ends_with('*') && !text.ends_with(last) {
        return false;
    }

    let mut pos = parts[0].len();
    for part in &parts[1..parts.len().saturating_sub(1)] {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(i) => pos += i + part.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn literal_exact() {
        assert!(matches("abc", "abc"));
        assert!(!matches("abc", "abd"));
        assert!(!matches("abc", "abcd"));
    }

    #[test]
    fn prefix_wildcard() {
        assert!(matches("deepseek*", "deepseek-v4"));
        assert!(!matches("deepseek*", "openai-gpt"));
    }

    #[test]
    fn suffix_wildcard() {
        assert!(matches("*-free", "claude-3-free"));
        assert!(matches("*free*", "xfreemium"));
        assert!(!matches("*-free", "claude-3"));
    }

    #[test]
    fn middle_wildcard() {
        assert!(matches("a*c", "abc"));
        assert!(matches("a*c", "aXYZc"));
        assert!(!matches("a*c", "abX"));
    }

    #[test]
    fn multiple_wildcards() {
        assert!(matches("*a*b*", "xaaybbz"));
        assert!(!matches("*a*b*", "xbbz"));
    }
}
