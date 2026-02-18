/// Represents a single segment of a superjson path.
///
/// Paths in superjson use dot notation: `"a.0.b"` means `obj["a"][0]["b"]`.
/// Each segment is either a string key (for objects) or a numeric index (for arrays).
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

/// Escape a key for use in a superjson dot-notation path.
///
/// Backslashes are escaped as `\\` and dots are escaped as `\.`.
/// This matches JS superjson's `escapeKey` behavior.
pub fn escape_key(key: &str) -> String {
    // Replace backslashes first, then dots (order matters)
    key.replace('\\', "\\\\").replace('.', "\\.")
}

/// Joins path segments into a superjson path string.
///
/// Keys containing dots or backslashes are escaped to avoid ambiguity
/// with the dot path separator.
///
/// # Examples
/// ```
/// use superjson_rs::path::{join, PathSegment};
///
/// assert_eq!(join(&[]), "");
/// assert_eq!(join(&[PathSegment::Key("a".into())]), "a");
/// assert_eq!(
///     join(&[PathSegment::Key("a".into()), PathSegment::Index(0), PathSegment::Key("b".into())]),
///     "a.0.b"
/// );
/// ```
pub fn join(segments: &[PathSegment]) -> String {
    let mut result = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            result.push('.');
        }
        match seg {
            PathSegment::Key(k) => result.push_str(&escape_key(k)),
            PathSegment::Index(idx) => result.push_str(&idx.to_string()),
        }
    }
    result
}

/// Parses a superjson path string into segments.
///
/// Splits on unescaped dots, then unescapes each segment.
/// `\.` is treated as a literal dot, `\\` is treated as a literal backslash.
///
/// # Examples
/// ```
/// use superjson_rs::path::{parse, PathSegment};
///
/// assert_eq!(parse(""), vec![]);
/// assert_eq!(parse("a"), vec![PathSegment::Key("a".into())]);
/// assert_eq!(
///     parse("a.0.b"),
///     vec![PathSegment::Key("a".into()), PathSegment::Index(0), PathSegment::Key("b".into())]
/// );
/// ```
pub fn parse(path: &str) -> Vec<PathSegment> {
    if path.is_empty() {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if next == '\\' || next == '.' {
                current.push(next);
                i += 2;
                continue;
            }
        }

        if ch == '.' {
            segments.push(make_segment(&current));
            current.clear();
            i += 1;
            continue;
        }

        current.push(ch);
        i += 1;
    }

    segments.push(make_segment(&current));
    segments
}

fn make_segment(s: &str) -> PathSegment {
    s.parse::<usize>()
        .map(PathSegment::Index)
        .unwrap_or_else(|_| PathSegment::Key(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_key_no_special_chars() {
        assert_eq!(escape_key("foo"), "foo");
    }

    #[test]
    fn test_escape_key_with_dot() {
        assert_eq!(escape_key("a.b"), "a\\.b");
    }

    #[test]
    fn test_escape_key_with_backslash() {
        assert_eq!(escape_key("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_escape_key_with_both() {
        assert_eq!(escape_key("a\\.b"), "a\\\\\\.b");
    }

    #[test]
    fn test_join_empty() {
        assert_eq!(join(&[]), "");
    }

    #[test]
    fn test_join_single_key() {
        assert_eq!(join(&[PathSegment::Key("foo".into())]), "foo");
    }

    #[test]
    fn test_join_nested() {
        let segments = vec![
            PathSegment::Key("a".into()),
            PathSegment::Index(0),
            PathSegment::Key("b".into()),
        ];
        assert_eq!(join(&segments), "a.0.b");
    }

    #[test]
    fn test_join_with_dot_in_key() {
        let segments = vec![PathSegment::Key("a.b".into()), PathSegment::Key("c".into())];
        assert_eq!(join(&segments), "a\\.b.c");
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse(""), Vec::<PathSegment>::new());
    }

    #[test]
    fn test_parse_single_key() {
        assert_eq!(parse("foo"), vec![PathSegment::Key("foo".into())]);
    }

    #[test]
    fn test_parse_nested() {
        assert_eq!(
            parse("a.0.b"),
            vec![
                PathSegment::Key("a".into()),
                PathSegment::Index(0),
                PathSegment::Key("b".into()),
            ]
        );
    }

    #[test]
    fn test_parse_with_escaped_dot() {
        assert_eq!(
            parse("a\\.b.c"),
            vec![PathSegment::Key("a.b".into()), PathSegment::Key("c".into()),]
        );
    }

    #[test]
    fn test_parse_with_escaped_backslash() {
        assert_eq!(
            parse("a\\\\b.c"),
            vec![
                PathSegment::Key("a\\b".into()),
                PathSegment::Key("c".into()),
            ]
        );
    }

    #[test]
    fn test_roundtrip() {
        let segments = vec![
            PathSegment::Key("data".into()),
            PathSegment::Index(2),
            PathSegment::Key("name".into()),
        ];
        assert_eq!(parse(&join(&segments)), segments);
    }

    #[test]
    fn test_roundtrip_with_special_chars() {
        let segments = vec![
            PathSegment::Key("a.b".into()),
            PathSegment::Index(0),
            PathSegment::Key("c\\d".into()),
        ];
        assert_eq!(parse(&join(&segments)), segments);
    }
}
