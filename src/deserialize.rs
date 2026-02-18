use chrono::DateTime;
use indexmap::IndexMap;
use num_bigint::BigInt;

use crate::error::Error;
use crate::{AnnotationValues, Result, SuperJson, TypeAnnotation, Value};

/// Deserialize a superjson `{json, meta}` representation back into a `Value`.
///
/// This restores extended types (Date, BigInt, Set, etc.) from their
/// JSON-compatible representations using the tree-structured type annotations.
pub fn deserialize(superjson: &SuperJson) -> Result<Value> {
    let values = superjson
        .meta
        .as_ref()
        .and_then(|m| m.values.as_ref());

    match values {
        Some(AnnotationValues::Root(ann)) => {
            deserialize_annotated(&superjson.json, ann)
        }
        Some(AnnotationValues::Children(children)) => {
            deserialize_with_children(&superjson.json, children)
        }
        None => deserialize_plain(&superjson.json),
    }
}

/// Deserialize a JSON value that has no annotation at all.
fn deserialize_plain(json: &serde_json::Value) -> Result<Value> {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => Ok(Value::Number(
            n.as_f64().ok_or_else(|| Error::TypeMismatch {
                path: String::new(),
                expected: "f64-compatible number".to_string(),
                actual: format!("{n}"),
            })?,
        )),
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<_>> = arr.iter().map(deserialize_plain).collect();
            Ok(Value::Array(values?))
        }
        serde_json::Value::Object(map) => {
            let mut obj = IndexMap::with_capacity(map.len());
            for (key, val) in map {
                obj.insert(key.clone(), deserialize_plain(val)?);
            }
            Ok(Value::Object(obj))
        }
    }
}

/// Deserialize a JSON value that has a direct type annotation.
fn deserialize_annotated(
    json: &serde_json::Value,
    annotation: &TypeAnnotation,
) -> Result<Value> {
    let type_name = annotation.type_name();
    let inner_children = annotation.children();

    match type_name {
        "undefined" => Ok(Value::Undefined),

        "Date" => {
            let s = expect_str(json, type_name)?;
            let dt = DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| Error::InvalidDate(format!("{s}: {e}")))?;
            Ok(Value::Date(dt))
        }

        "bigint" => {
            let s = expect_str(json, type_name)?;
            let n: BigInt = s
                .parse()
                .map_err(|e| Error::InvalidBigInt(format!("{s}: {e}")))?;
            Ok(Value::BigInt(n))
        }

        "set" => {
            let arr = expect_array(json, type_name)?;
            let empty = IndexMap::new();
            let children = inner_children.unwrap_or(&empty);
            let mut items = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                let key = i.to_string();
                items.push(deserialize_child(item, &key, children)?);
            }
            Ok(Value::Set(items))
        }

        "map" => {
            let arr = expect_array(json, type_name)?;
            let empty = IndexMap::new();
            let children = inner_children.unwrap_or(&empty);
            let mut entries = Vec::with_capacity(arr.len());
            for (i, entry) in arr.iter().enumerate() {
                let pair = entry.as_array().ok_or_else(|| Error::TypeMismatch {
                    path: format!("{i}"),
                    expected: "array (key-value pair)".to_string(),
                    actual: format!("{entry}"),
                })?;
                if pair.len() != 2 {
                    return Err(Error::TypeMismatch {
                        path: format!("{i}"),
                        expected: "array of length 2".to_string(),
                        actual: format!("array of length {}", pair.len()),
                    });
                }

                let key_path = format!("{i}.0");
                let val_path = format!("{i}.1");
                let key = deserialize_child(&pair[0], &key_path, children)?;
                let val = deserialize_child(&pair[1], &val_path, children)?;
                entries.push((key, val));
            }
            Ok(Value::Map(entries))
        }

        "number" => {
            let s = expect_str(json, type_name)?;
            match s {
                "NaN" => Ok(Value::NaN),
                "Infinity" => Ok(Value::PosInfinity),
                "-Infinity" => Ok(Value::NegInfinity),
                _ => Err(Error::TypeMismatch {
                    path: String::new(),
                    expected: "NaN, Infinity, or -Infinity".to_string(),
                    actual: s.to_string(),
                }),
            }
        }

        "regexp" => {
            let s = expect_str(json, type_name)?;
            parse_regexp(s)
        }

        _ => Err(Error::InvalidTypeAnnotation(format!(
            "unknown type '{type_name}'"
        ))),
    }
}

/// Deserialize a JSON value (object or array) whose children have annotations.
fn deserialize_with_children(
    json: &serde_json::Value,
    children: &IndexMap<String, TypeAnnotation>,
) -> Result<Value> {
    match json {
        serde_json::Value::Array(arr) => {
            let mut values = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                let key = i.to_string();
                values.push(deserialize_child(item, &key, children)?);
            }
            Ok(Value::Array(values))
        }
        serde_json::Value::Object(map) => {
            let mut obj = IndexMap::with_capacity(map.len());
            for (key, val) in map {
                obj.insert(key.clone(), deserialize_child(val, key, children)?);
            }
            Ok(Value::Object(obj))
        }
        _ => {
            // If no annotation matches, deserialize as plain
            deserialize_plain(json)
        }
    }
}

/// Deserialize a child value by looking up its annotation in the children map.
///
/// The children map uses dot-notation paths. A child at key "foo" might have:
/// - A direct annotation: `children["foo"] = Leaf("Date")`
/// - Sub-children annotations: `children["foo.bar"] = Leaf("Date")`
/// - No annotation: deserialize as plain JSON
fn deserialize_child(
    json: &serde_json::Value,
    key: &str,
    children: &IndexMap<String, TypeAnnotation>,
) -> Result<Value> {
    // Check for a direct annotation on this child
    if let Some(ann) = children.get(key) {
        return deserialize_annotated(json, ann);
    }

    // Check for sub-children annotations (paths starting with "key.")
    let prefix = format!("{key}.");
    let sub_children: IndexMap<String, TypeAnnotation> = children
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix(&prefix)
                .map(|rest| (rest.to_string(), v.clone()))
        })
        .collect();

    if !sub_children.is_empty() {
        return deserialize_with_children(json, &sub_children);
    }

    // No annotation - deserialize as plain JSON
    deserialize_plain(json)
}

fn expect_str<'a>(json: &'a serde_json::Value, type_name: &str) -> Result<&'a str> {
    json.as_str().ok_or_else(|| Error::TypeMismatch {
        path: String::new(),
        expected: format!("string for {type_name}"),
        actual: format!("{json}"),
    })
}

fn expect_array<'a>(
    json: &'a serde_json::Value,
    type_name: &str,
) -> Result<&'a Vec<serde_json::Value>> {
    json.as_array().ok_or_else(|| Error::TypeMismatch {
        path: String::new(),
        expected: format!("array for {type_name}"),
        actual: format!("{json}"),
    })
}

/// Parse a regexp string like "/pattern/flags" into a `Value::RegExp`.
fn parse_regexp(s: &str) -> Result<Value> {
    if !s.starts_with('/') {
        return Err(Error::InvalidRegExp(format!(
            "regexp must start with '/': {s}"
        )));
    }

    let last_slash = s.rfind('/').filter(|&i| i > 0).ok_or_else(|| {
        Error::InvalidRegExp(format!("regexp must have closing '/': {s}"))
    })?;

    let source = s[1..last_slash].to_string();
    let flags = s[last_slash + 1..].to_string();

    Ok(Value::RegExp { source, flags })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Meta;
    use chrono::TimeZone;
    use serde_json::json;

    fn make_superjson_root(json: serde_json::Value, ann: TypeAnnotation) -> SuperJson {
        SuperJson {
            json,
            meta: Some(Meta {
                values: Some(AnnotationValues::Root(ann)),
                referential_equalities: None,
                v: Some(1),
            }),
        }
    }

    fn make_superjson_children(
        json: serde_json::Value,
        children: IndexMap<String, TypeAnnotation>,
    ) -> SuperJson {
        SuperJson {
            json,
            meta: Some(Meta {
                values: Some(AnnotationValues::Children(children)),
                referential_equalities: None,
                v: Some(1),
            }),
        }
    }

    fn make_superjson_plain(json: serde_json::Value) -> SuperJson {
        SuperJson { json, meta: None }
    }

    #[test]
    fn test_deserialize_null() {
        let sj = make_superjson_plain(json!(null));
        assert_eq!(deserialize(&sj).unwrap(), Value::Null);
    }

    #[test]
    fn test_deserialize_bool() {
        let sj = make_superjson_plain(json!(true));
        assert_eq!(deserialize(&sj).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_deserialize_number() {
        let sj = make_superjson_plain(json!(42.0));
        assert_eq!(deserialize(&sj).unwrap(), Value::Number(42.0));
    }

    #[test]
    fn test_deserialize_string() {
        let sj = make_superjson_plain(json!("hello"));
        assert_eq!(deserialize(&sj).unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_deserialize_undefined() {
        let sj = make_superjson_root(json!(null), TypeAnnotation::Leaf("undefined".into()));
        assert_eq!(deserialize(&sj).unwrap(), Value::Undefined);
    }

    #[test]
    fn test_deserialize_date() {
        let sj = make_superjson_root(
            json!("1970-01-01T00:00:00.000Z"),
            TypeAnnotation::Leaf("Date".into()),
        );
        let expected = Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap());
        assert_eq!(deserialize(&sj).unwrap(), expected);
    }

    #[test]
    fn test_deserialize_bigint() {
        let sj = make_superjson_root(json!("42"), TypeAnnotation::Leaf("bigint".into()));
        assert_eq!(deserialize(&sj).unwrap(), Value::BigInt(BigInt::from(42)));
    }

    #[test]
    fn test_deserialize_set() {
        let sj = make_superjson_root(
            json!([1.0, 2.0, 3.0]),
            TypeAnnotation::Leaf("set".into()),
        );
        assert_eq!(
            deserialize(&sj).unwrap(),
            Value::Set(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ])
        );
    }

    #[test]
    fn test_deserialize_set_with_inner_annotations() {
        let mut inner = IndexMap::new();
        inner.insert("1".to_string(), TypeAnnotation::Leaf("undefined".into()));
        let sj = make_superjson_root(
            json!([1.0, null, 2.0]),
            TypeAnnotation::Node("set".into(), inner),
        );
        assert_eq!(
            deserialize(&sj).unwrap(),
            Value::Set(vec![
                Value::Number(1.0),
                Value::Undefined,
                Value::Number(2.0),
            ])
        );
    }

    #[test]
    fn test_deserialize_map() {
        let sj = make_superjson_root(
            json!([["key", 1.0]]),
            TypeAnnotation::Leaf("map".into()),
        );
        assert_eq!(
            deserialize(&sj).unwrap(),
            Value::Map(vec![(Value::String("key".into()), Value::Number(1.0))])
        );
    }

    #[test]
    fn test_deserialize_map_with_nan_key() {
        let mut inner = IndexMap::new();
        inner.insert("0.0".to_string(), TypeAnnotation::Leaf("number".into()));
        let sj = make_superjson_root(
            json!([["NaN", null]]),
            TypeAnnotation::Node("map".into(), inner),
        );
        assert_eq!(
            deserialize(&sj).unwrap(),
            Value::Map(vec![(Value::NaN, Value::Null)])
        );
    }

    #[test]
    fn test_deserialize_nan() {
        let sj = make_superjson_root(json!("NaN"), TypeAnnotation::Leaf("number".into()));
        assert_eq!(deserialize(&sj).unwrap(), Value::NaN);
    }

    #[test]
    fn test_deserialize_infinity() {
        let sj = make_superjson_root(json!("Infinity"), TypeAnnotation::Leaf("number".into()));
        assert_eq!(deserialize(&sj).unwrap(), Value::PosInfinity);

        let sj = make_superjson_root(json!("-Infinity"), TypeAnnotation::Leaf("number".into()));
        assert_eq!(deserialize(&sj).unwrap(), Value::NegInfinity);
    }

    #[test]
    fn test_deserialize_regexp() {
        let sj = make_superjson_root(json!("/\\d+/gi"), TypeAnnotation::Leaf("regexp".into()));
        assert_eq!(
            deserialize(&sj).unwrap(),
            Value::RegExp {
                source: "\\d+".into(),
                flags: "gi".into(),
            }
        );
    }

    #[test]
    fn test_deserialize_object_with_children() {
        let mut children = IndexMap::new();
        children.insert("created".to_string(), TypeAnnotation::Leaf("Date".into()));

        let sj = make_superjson_children(
            json!({"created": "1970-01-01T00:00:00.000Z", "name": "test"}),
            children,
        );
        let result = deserialize(&sj).unwrap();
        let obj = result.as_object().unwrap();
        assert_eq!(
            obj.get("created").unwrap(),
            &Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap())
        );
        assert_eq!(obj.get("name").unwrap(), &Value::String("test".into()));
    }

    #[test]
    fn test_deserialize_deeply_nested() {
        let mut children = IndexMap::new();
        children.insert(
            "meeting.date".to_string(),
            TypeAnnotation::Leaf("Date".into()),
        );

        let sj = make_superjson_children(
            json!({"meeting": {"date": "1970-01-01T00:00:00.000Z"}}),
            children,
        );
        let result = deserialize(&sj).unwrap();
        let meeting = result.as_object().unwrap().get("meeting").unwrap();
        let date = meeting.as_object().unwrap().get("date").unwrap();
        assert_eq!(
            date,
            &Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap())
        );
    }

    #[test]
    fn test_deserialize_array_with_children() {
        let mut children = IndexMap::new();
        children.insert("1".to_string(), TypeAnnotation::Leaf("Date".into()));
        children.insert("2".to_string(), TypeAnnotation::Leaf("bigint".into()));

        let sj = make_superjson_children(
            json!([1.0, "1970-01-01T00:00:00.000Z", "999"]),
            children,
        );
        let result = deserialize(&sj).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0], Value::Number(1.0));
        assert_eq!(
            arr[1],
            Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap())
        );
        assert_eq!(arr[2], Value::BigInt(BigInt::from(999)));
    }
}
