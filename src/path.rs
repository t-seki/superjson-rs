/// Represents a single segment of a superjson path.
///
/// Paths in superjson use dot notation: `"a.0.b"` means `obj["a"][0]["b"]`.
/// Each segment is either a string key (for objects) or a numeric index (for arrays).
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

/// Joins path segments into a superjson path string.
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
            PathSegment::Key(k) => result.push_str(k),
            PathSegment::Index(idx) => result.push_str(&idx.to_string()),
        }
    }
    result
}

/// Parses a superjson path string into segments.
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
    path.split('.')
        .map(|s| {
            s.parse::<usize>()
                .map(PathSegment::Index)
                .unwrap_or_else(|_| PathSegment::Key(s.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_roundtrip() {
        let segments = vec![
            PathSegment::Key("data".into()),
            PathSegment::Index(2),
            PathSegment::Key("name".into()),
        ];
        assert_eq!(parse(&join(&segments)), segments);
    }
}
