//! Surgical JSONC patcher.
//!
//! Guarantees: replaces or inserts a single value at a JSON path while
//! preserving every byte outside that span — comments, whitespace, key
//! order, trailing commas, CRLF, and BOM are all left untouched.

use anyhow::{bail, Result};
use serde_json::Value;

/// Upsert `value` at `path` in the JSONC document `src`. Missing intermediate
/// parents are materialized as empty objects. Returns the new document.
pub fn upsert(src: &str, path: &[&str], value: &Value) -> Result<String> {
    if path.is_empty() {
        bail!("empty path");
    }
    let mut current = src.to_string();

    // Materialize any missing intermediate parents as empty objects.
    for i in 1..path.len() {
        let prefix: Vec<&str> = path[..i].to_vec();
        if find_value_span(current.as_bytes(), &prefix)?.is_none() {
            current = insert_value(&current, &prefix, &Value::Object(Default::default()))?;
        }
    }

    // Leaf: replace if present, else insert.
    match find_value_span(current.as_bytes(), path)? {
        Some((s, e)) => Ok(replace_span(&current, s, e, value)),
        None => insert_value(&current, path, value),
    }
}

/// Locate the byte span `(start, end)` of the value at `path`, if present.
/// `start` is the first byte of the value; `end` is one past the last byte.
pub fn find_value_span(src: &[u8], path: &[&str]) -> Result<Option<(usize, usize)>> {
    let mut cur = Cursor::new(src);
    walk(&mut cur, path)
}

fn walk(cur: &mut Cursor, path: &[&str]) -> Result<Option<(usize, usize)>> {
    cur.skip_ws_and_comments();
    if cur.peek() != Some(b'{') {
        return Ok(None);
    }
    cur.bump(); // consume `{`
    let target = path[0];
    let rest = &path[1..];
    loop {
        cur.skip_ws_and_comments();
        match cur.peek() {
            Some(b'}') => {
                cur.bump();
                return Ok(None);
            }
            Some(b',') => {
                cur.bump();
            }
            Some(b'"') => {
                let key = cur.read_string()?;
                cur.skip_ws_and_comments();
                if cur.bump() != Some(b':') {
                    bail!("expected ':' at pos {}", cur.pos);
                }
                cur.skip_ws_and_comments();
                if key == target {
                    if rest.is_empty() {
                        let start = cur.pos;
                        cur.skip_value()?;
                        return Ok(Some((start, cur.pos)));
                    }
                    return walk(cur, rest);
                }
                cur.skip_value()?;
            }
            _ => bail!("expected key or '}}' at pos {}", cur.pos),
        }
    }
}

fn replace_span(src: &str, start: usize, end: usize, value: &Value) -> String {
    let original = &src[start..end];
    let was_inline = !original.contains('\n');
    let indent = line_indent(src, start);

    // Keep simple scalar / single-field values compact when they were
    // originally inline, but pretty-print anything larger so generated
    // files stay readable.
    let serialized = if was_inline && is_simple_value(value) {
        serde_json::to_string(value).expect("serialize")
    } else {
        reindent(
            &serde_json::to_string_pretty(value).expect("serialize"),
            &indent,
        )
    };

    let mut out = String::with_capacity(src.len() + serialized.len());
    out.push_str(&src[..start]);
    out.push_str(&serialized);
    out.push_str(&src[end..]);
    out
}

fn is_simple_value(value: &Value) -> bool {
    match value {
        Value::Object(m) => m.len() <= 1 && m.values().all(is_simple_value),
        Value::Array(a) => a.len() <= 1 && a.iter().all(is_simple_value),
        _ => true,
    }
}

fn insert_value(src: &str, path: &[&str], value: &Value) -> Result<String> {
    let (parent_start, parent_end) = if path.len() == 1 {
        find_root_object_span(src)?
    } else {
        find_value_span(src.as_bytes(), &path[..path.len() - 1])?
            .ok_or_else(|| anyhow::anyhow!("missing parent path {:?}", &path[..path.len() - 1]))?
    };

    if src.as_bytes().get(parent_start) != Some(&b'{') {
        bail!("parent at {:?} is not an object", &path[..path.len() - 1]);
    }
    if parent_end <= parent_start + 1 || src.as_bytes().get(parent_end - 1) != Some(&b'}') {
        bail!("parent at {:?} has unexpected span", &path[..path.len() - 1]);
    }

    let close_pos = parent_end - 1;
    let content_slice = &src[parent_start..close_pos];
    let last = find_last_content_end(content_slice)?;

    let key = path.last().unwrap();
    let parent_indent = line_indent(src, parent_start);
    let inner_indent = format!("{}  ", parent_indent);
    let reindented = reindent(
        &serde_json::to_string_pretty(value)?,
        &inner_indent,
    );
    let entry = format!("{}: {}", serde_json::to_string(key)?, reindented);

    let mut out = String::with_capacity(src.len() + entry.len() + 32);
    match last {
        Some(off) => {
            // Insert after the last value; comma-prefix and re-indent.
            let abs = parent_start + off;
            out.push_str(&src[..abs]);
            out.push_str(",\n");
            out.push_str(&inner_indent);
            out.push_str(&entry);
            out.push_str(&src[abs..]);
        }
        None => {
            // Empty object: replace the content between `{` and `}`.
            out.push_str(&src[..parent_start + 1]);
            out.push('\n');
            out.push_str(&inner_indent);
            out.push_str(&entry);
            out.push('\n');
            out.push_str(&parent_indent);
            out.push_str(&src[close_pos..]);
        }
    }
    Ok(out)
}

/// Return the byte offset within `slice` (which begins with `{` and does NOT
/// include the closing `}`) immediately after the last key/value pair. Returns
/// `None` if the object is empty (only whitespace/comments between braces).
fn find_last_content_end(slice: &str) -> Result<Option<usize>> {
    let mut cur = Cursor::new(slice.as_bytes());
    cur.pos = 1; // skip the leading `{`
    let mut last_end = None;
    loop {
        cur.skip_ws_and_comments();
        match cur.peek() {
            Some(b',') => {
                cur.bump();
            }
            Some(b'"') => {
                cur.read_string()?;
                cur.skip_ws_and_comments();
                if cur.bump() != Some(b':') {
                    bail!("expected ':' in object at pos {}", cur.pos);
                }
                cur.skip_ws_and_comments();
                cur.skip_value()?;
                last_end = Some(cur.pos);
            }
            None | Some(b'}') => return Ok(last_end),
            _ => bail!("unexpected byte in object at pos {}", cur.pos),
        }
    }
}

fn find_root_object_span(src: &str) -> Result<(usize, usize)> {
    let bytes = src.as_bytes();
    let mut cursor = Cursor::new(bytes);
    // Skip a UTF-8 BOM if present.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        cursor.pos = 3;
    }
    cursor.skip_ws_and_comments();
    let obj_start = cursor.pos;
    if bytes.get(obj_start) != Some(&b'{') {
        bail!("root is not an object");
    }
    cursor.skip_container(b'}')?;
    Ok((obj_start, cursor.pos))
}

fn line_indent(src: &str, pos: usize) -> String {
    let line_start = src[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    src[line_start..pos]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

fn reindent(pretty: &str, prefix: &str) -> String {
    let mut out = String::new();
    for (i, line) in pretty.lines().enumerate() {
        if i > 0 {
            out.push('\n');
            out.push_str(prefix);
        }
        out.push_str(line);
    }
    out
}

// ── core scanner ───────────────────────────────────────────────────────

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => self.pos += 1,
                Some(b'/') => match self.bytes.get(self.pos + 1).copied() {
                    Some(b'/') => {
                        self.pos += 2;
                        while let Some(b) = self.peek() {
                            if b == b'\n' {
                                break;
                            }
                            self.pos += 1;
                        }
                    }
                    Some(b'*') => {
                        self.pos += 2;
                        while self.pos + 1 < self.bytes.len() {
                            if self.bytes[self.pos] == b'*'
                                && self.bytes[self.pos + 1] == b'/'
                            {
                                self.pos += 2;
                                break;
                            }
                            self.pos += 1;
                        }
                    }
                    _ => return,
                },
                _ => return,
            }
        }
    }

    fn read_string(&mut self) -> Result<String> {
        if self.bump() != Some(b'"') {
            bail!("expected string at pos {}", self.pos);
        }
        let start = self.pos;
        loop {
            match self.bump() {
                Some(b'\\') => {
                    self.bump();
                }
                Some(b'"') => {
                    let end = self.pos - 1;
                    return Ok(unescape(std::str::from_utf8(&self.bytes[start..end])?));
                }
                Some(_) => {}
                None => bail!("unterminated string"),
            }
        }
    }

    fn skip_value(&mut self) -> Result<()> {
        self.skip_ws_and_comments();
        match self.peek() {
            Some(b'"') => {
                self.read_string()?;
                Ok(())
            }
            Some(b'{') => self.skip_container(b'}'),
            Some(b'[') => self.skip_container(b']'),
            Some(b't') | Some(b'f') | Some(b'n') => {
                while matches!(self.peek(), Some(b) if b.is_ascii_alphabetic()) {
                    self.bump();
                }
                Ok(())
            }
            Some(b'-') | Some(b'0'..=b'9') => {
                while matches!(self.peek(),
                    Some(b) if b.is_ascii_alphanumeric()
                        || b == b'.' || b == b'+' || b == b'-'
                        || b == b'e' || b == b'E')
                {
                    self.bump();
                }
                Ok(())
            }
            _ => bail!("unexpected value at pos {}", self.pos),
        }
    }

    fn skip_container(&mut self, close: u8) -> Result<()> {
        self.bump(); // consume opening brace/bracket
        loop {
            self.skip_ws_and_comments();
            match self.peek() {
                Some(b) if b == close => {
                    self.bump();
                    return Ok(());
                }
                Some(b'"') => {
                    self.read_string()?;
                }
                Some(b'{') => self.skip_container(b'}')?,
                Some(b'[') => self.skip_container(b']')?,
                Some(_) => {
                    self.bump();
                }
                None => bail!("unterminated container"),
            }
        }
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some(x) => {
                    out.push('\\');
                    out.push(x);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replace_preserves_comments() {
        let src = r#"{
  // important note
  "provider": {
    "other": 1,
    "nanogpt": { "old": true } // trailing
  },
  /* block */
  "keep": "me"
}"#;
        let out = upsert(src, &["provider", "nanogpt"], &json!({"new": true})).unwrap();
        assert!(out.contains("// important note"));
        assert!(out.contains("// trailing"));
        assert!(out.contains("/* block */"));
        assert!(out.contains("\"keep\": \"me\""));
        assert!(out.contains("new"));
        assert!(!out.contains("old"));
    }

    #[test]
    fn replace_preserves_multiline_layout() {
        let src = "{\n  \"nanogpt\": {\n    \"old\": true\n  }\n}\n";
        let out = upsert(src, &["nanogpt"], &json!({"new": true})).unwrap();
        assert_eq!(out, "{\n  \"nanogpt\": {\n    \"new\": true\n  }\n}\n");
    }

    #[test]
    fn insert_into_empty_root() {
        let src = "{\n}\n";
        let out = upsert(src, &["provider", "nanogpt"], &json!({"a": 1})).unwrap();
        assert!(out.contains("\"provider\": {"));
        assert!(out.contains("\"nanogpt\": {"));
        assert!(out.contains("\"a\": 1"));
        assert!(out.ends_with("}\n"));
    }

    #[test]
    fn insert_preserves_crlf_and_comments() {
        let src = "{\r\n  // c\r\n  \"keep\": true\r\n}\r\n";
        let out = upsert(src, &["added"], &json!(42)).unwrap();
        assert!(out.contains("// c"));
        assert!(out.contains("\"added\": 42"));
        assert!(out.contains("\"keep\": true"));
        assert!(out.starts_with("{\r\n"));
    }

    #[test]
    fn find_span_returns_none_for_missing_path() {
        let src = r#"{"a": {"b": 1}}"#;
        assert!(find_value_span(src.as_bytes(), &["a", "c"]).unwrap().is_none());
    }
}
