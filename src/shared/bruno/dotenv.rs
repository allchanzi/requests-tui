use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Load the collection's `.env` file (Bruno keeps secrets here, gitignored). The file
/// is optional; a missing or unreadable file yields an empty map. Keys are exposed to
/// requests as `process.env.<key>`, matching Bruno's `{{process.env.X}}` references.
pub fn load(dir: &Path) -> HashMap<String, String> {
    match fs::read_to_string(dir.join(".env")) {
        Ok(raw) => parse(&raw),
        Err(_) => HashMap::new(),
    }
}

/// Parse the contents of a `.env` file. Supports `KEY=VALUE` lines, `#` comments, an
/// optional `export ` prefix, and single/double-quoted values. Double-quoted values
/// honour `\n`, `\r`, `\t`, `\\` and `\"` escapes; unquoted values drop a trailing
/// ` # comment`. Later assignments to the same key win.
pub fn parse(input: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line
            .strip_prefix("export ")
            .map(str::trim_start)
            .unwrap_or(line);

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        values.insert(key.to_string(), parse_value(raw_value.trim()));
    }

    values
}

fn parse_value(raw: &str) -> String {
    if let Some(inner) = strip_quotes(raw, '"') {
        return unescape(inner);
    }
    if let Some(inner) = strip_quotes(raw, '\'') {
        return inner.to_string();
    }
    strip_inline_comment(raw).trim_end().to_string()
}

/// Return the content inside a matching pair of `quote` characters, or `None` if `raw`
/// is not wrapped in them.
fn strip_quotes(raw: &str, quote: char) -> Option<&str> {
    let mut chars = raw.chars();
    if raw.chars().count() >= 2 && chars.next() == Some(quote) && chars.next_back() == Some(quote) {
        Some(chars.as_str())
    } else {
        None
    }
}

/// Trim a trailing inline comment introduced by ` #` (whitespace then hash) so values
/// that legitimately contain `#` (tokens, URL fragments) are left intact.
fn strip_inline_comment(raw: &str) -> &str {
    match raw.find(" #") {
        Some(index) => &raw[..index],
        None => raw,
    }
}

fn unescape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_pairs_skipping_comments_and_blanks() {
        let env = parse("# comment\n\nAPI_KEY=abc123\nexport TOKEN = xyz\n");
        assert_eq!(env.get("API_KEY").map(String::as_str), Some("abc123"));
        assert_eq!(env.get("TOKEN").map(String::as_str), Some("xyz"));
    }

    #[test]
    fn handles_quotes_escapes_and_inline_comments() {
        let env = parse(
            "DQ=\"line1\\nline2\"\nSQ='raw\\nvalue'\nBARE=plain # trailing\nHASH=ab#cd\n",
        );
        assert_eq!(env.get("DQ").map(String::as_str), Some("line1\nline2"));
        // Single quotes are literal: no escape expansion.
        assert_eq!(env.get("SQ").map(String::as_str), Some("raw\\nvalue"));
        assert_eq!(env.get("BARE").map(String::as_str), Some("plain"));
        // A `#` without preceding whitespace is part of the value.
        assert_eq!(env.get("HASH").map(String::as_str), Some("ab#cd"));
    }

    #[test]
    fn last_assignment_wins_and_empty_keys_ignored() {
        let env = parse("X=1\nX=2\n=novalue\n");
        assert_eq!(env.get("X").map(String::as_str), Some("2"));
        assert_eq!(env.len(), 1);
    }
}
