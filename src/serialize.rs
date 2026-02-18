use chrono::SecondsFormat;
use indexmap::IndexMap;
use serde_json::json;

use crate::{AnnotationValues, Meta, Result, SuperJson, TypeAnnotation, Value};

/// Internal result from serializing a value.
/// Tracks whether the annotation applies to the value itself or its children.
enum AnnotationResult {
    /// This value itself is an extended type (Date, Set, etc.)
    Typed(TypeAnnotation),
    /// This value is a plain object/array with annotated descendants.
    Children(IndexMap<String, TypeAnnotation>),
}

/// Serialize a `Value` into the superjson `{json, meta}` representation.
///
/// This converts extended types (Date, BigInt, Set, etc.) into JSON-compatible
/// values and records type annotations in the metadata using the tree format
/// compatible with JS superjson.
pub fn serialize(value: &Value) -> Result<SuperJson> {
    let (json, annotation) = serialize_value(value)?;

    let meta = annotation.map(|ann| {
        let values = match ann {
            AnnotationResult::Typed(t) => AnnotationValues::Root(t),
            AnnotationResult::Children(c) => AnnotationValues::Children(c),
        };
        Meta {
            values: Some(values),
            referential_equalities: None,
            v: Some(1),
        }
    });

    Ok(SuperJson { json, meta })
}

fn serialize_value(value: &Value) -> Result<(serde_json::Value, Option<AnnotationResult>)> {
    match value {
        // Standard JSON types - no annotation needed
        Value::Null => Ok((serde_json::Value::Null, None)),
        Value::Bool(b) => Ok((json!(*b), None)),
        Value::Number(n) => Ok((json!(*n), None)),
        Value::String(s) => Ok((json!(s), None)),

        Value::Array(arr) => serialize_container_children(arr.iter(), ContainerKind::Array),
        Value::Object(map) => {
            let mut json_map = serde_json::Map::new();
            let mut children = IndexMap::new();

            for (key, val) in map {
                let (json_val, ann) = serialize_value(val)?;
                json_map.insert(key.clone(), json_val);
                collect_child_annotation(&mut children, key, ann);
            }

            let annotation = if children.is_empty() {
                None
            } else {
                Some(AnnotationResult::Children(children))
            };
            Ok((serde_json::Value::Object(json_map), annotation))
        }

        // Extended types - require annotation
        Value::Undefined => Ok((serde_json::Value::Null, Some(leaf("undefined")))),

        Value::Date(dt) => {
            let s = dt.to_rfc3339_opts(SecondsFormat::Millis, true);
            Ok((json!(s), Some(leaf("Date"))))
        }

        Value::BigInt(n) => Ok((json!(n.to_string()), Some(leaf("bigint")))),

        Value::Set(items) => {
            let (json_val, inner) =
                serialize_container_inner(items.iter(), ContainerKind::Array)?;
            let annotation = make_typed_annotation("set", inner);
            Ok((json_val, Some(annotation)))
        }

        Value::Map(entries) => {
            let mut json_arr = Vec::with_capacity(entries.len());
            let mut inner_children = IndexMap::new();

            for (i, (k, v)) in entries.iter().enumerate() {
                let (json_key, key_ann) = serialize_value(k)?;
                let (json_val, val_ann) = serialize_value(v)?;

                json_arr.push(json!([json_key, json_val]));

                let i_str = i.to_string();
                collect_child_annotation(&mut inner_children, &format!("{i_str}.0"), key_ann);
                collect_child_annotation(&mut inner_children, &format!("{i_str}.1"), val_ann);
            }

            let annotation = make_typed_annotation("map", inner_children);
            Ok((serde_json::Value::Array(json_arr), Some(annotation)))
        }

        Value::NaN => Ok((json!("NaN"), Some(leaf("number")))),
        Value::PosInfinity => Ok((json!("Infinity"), Some(leaf("number")))),
        Value::NegInfinity => Ok((json!("-Infinity"), Some(leaf("number")))),

        Value::RegExp { source, flags } => {
            Ok((json!(format!("/{source}/{flags}")), Some(leaf("regexp"))))
        }
    }
}

#[derive(Clone, Copy)]
enum ContainerKind {
    Array,
}

/// Serialize children of a plain container (Array) and collect annotations.
/// Returns the JSON value and an optional AnnotationResult::Children.
fn serialize_container_children<'a>(
    items: impl Iterator<Item = &'a Value>,
    _kind: ContainerKind,
) -> Result<(serde_json::Value, Option<AnnotationResult>)> {
    let mut json_arr = Vec::new();
    let mut children = IndexMap::new();

    for (i, item) in items.enumerate() {
        let (json_val, ann) = serialize_value(item)?;
        json_arr.push(json_val);
        collect_child_annotation(&mut children, &i.to_string(), ann);
    }

    let annotation = if children.is_empty() {
        None
    } else {
        Some(AnnotationResult::Children(children))
    };
    Ok((serde_json::Value::Array(json_arr), annotation))
}

/// Serialize children of an extended container (Set) and collect inner annotations.
/// Returns the JSON value and the inner children map (for use in TypeAnnotation::Node).
fn serialize_container_inner<'a>(
    items: impl Iterator<Item = &'a Value>,
    _kind: ContainerKind,
) -> Result<(serde_json::Value, IndexMap<String, TypeAnnotation>)> {
    let mut json_arr = Vec::new();
    let mut inner_children = IndexMap::new();

    for (i, item) in items.enumerate() {
        let (json_val, ann) = serialize_value(item)?;
        json_arr.push(json_val);
        collect_child_annotation(&mut inner_children, &i.to_string(), ann);
    }

    Ok((serde_json::Value::Array(json_arr), inner_children))
}

/// Collect a child's annotation into a parent's children map.
///
/// - If the child is a `Typed` annotation, insert it directly at `key`.
/// - If the child has `Children`, flatten them by prepending `key.` to each path.
fn collect_child_annotation(
    children: &mut IndexMap<String, TypeAnnotation>,
    key: &str,
    annotation: Option<AnnotationResult>,
) {
    match annotation {
        None => {}
        Some(AnnotationResult::Typed(t)) => {
            children.insert(key.to_string(), t);
        }
        Some(AnnotationResult::Children(inner)) => {
            for (inner_key, inner_ann) in inner {
                children.insert(format!("{key}.{inner_key}"), inner_ann);
            }
        }
    }
}

fn leaf(type_name: &str) -> AnnotationResult {
    AnnotationResult::Typed(TypeAnnotation::Leaf(type_name.to_string()))
}

fn make_typed_annotation(
    type_name: &str,
    inner: IndexMap<String, TypeAnnotation>,
) -> AnnotationResult {
    if inner.is_empty() {
        AnnotationResult::Typed(TypeAnnotation::Leaf(type_name.to_string()))
    } else {
        AnnotationResult::Typed(TypeAnnotation::Node(type_name.to_string(), inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use num_bigint::BigInt;

    #[test]
    fn test_serialize_null() {
        let result = serialize(&Value::Null).unwrap();
        assert_eq!(result.json, json!(null));
        assert!(result.meta.is_none());
    }

    #[test]
    fn test_serialize_bool() {
        let result = serialize(&Value::Bool(true)).unwrap();
        assert_eq!(result.json, json!(true));
        assert!(result.meta.is_none());
    }

    #[test]
    fn test_serialize_number() {
        let result = serialize(&Value::Number(42.0)).unwrap();
        assert_eq!(result.json, json!(42.0));
        assert!(result.meta.is_none());
    }

    #[test]
    fn test_serialize_string() {
        let result = serialize(&Value::String("hello".into())).unwrap();
        assert_eq!(result.json, json!("hello"));
        assert!(result.meta.is_none());
    }

    #[test]
    fn test_serialize_date() {
        let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
        let result = serialize(&Value::Date(dt)).unwrap();
        assert_eq!(result.json, json!("1970-01-01T00:00:00.000Z"));
        let meta = result.meta.unwrap();
        assert_eq!(meta.v, Some(1));
        assert_eq!(
            meta.values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("Date".into()))
        );
    }

    #[test]
    fn test_serialize_bigint() {
        let result = serialize(&Value::BigInt(BigInt::from(42))).unwrap();
        assert_eq!(result.json, json!("42"));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("bigint".into()))
        );
    }

    #[test]
    fn test_serialize_undefined() {
        let result = serialize(&Value::Undefined).unwrap();
        assert_eq!(result.json, json!(null));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("undefined".into()))
        );
    }

    #[test]
    fn test_serialize_set_simple() {
        let result = serialize(&Value::Set(vec![
            Value::Number(1.0),
            Value::Number(2.0),
        ]))
        .unwrap();
        assert_eq!(result.json, json!([1.0, 2.0]));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("set".into()))
        );
    }

    #[test]
    fn test_serialize_set_with_extended_children() {
        let result = serialize(&Value::Set(vec![
            Value::Number(1.0),
            Value::Undefined,
            Value::Number(2.0),
        ]))
        .unwrap();
        assert_eq!(result.json, json!([1.0, null, 2.0]));
        let mut expected_inner = IndexMap::new();
        expected_inner.insert("1".to_string(), TypeAnnotation::Leaf("undefined".into()));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Node("set".into(), expected_inner))
        );
    }

    #[test]
    fn test_serialize_map() {
        let result = serialize(&Value::Map(vec![(
            Value::String("key".into()),
            Value::Number(1.0),
        )]))
        .unwrap();
        assert_eq!(result.json, json!([["key", 1.0]]));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("map".into()))
        );
    }

    #[test]
    fn test_serialize_map_with_nan_key() {
        let result = serialize(&Value::Map(vec![(Value::NaN, Value::Null)])).unwrap();
        assert_eq!(result.json, json!([["NaN", null]]));
        let mut expected_inner = IndexMap::new();
        expected_inner.insert("0.0".to_string(), TypeAnnotation::Leaf("number".into()));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Node("map".into(), expected_inner))
        );
    }

    #[test]
    fn test_serialize_nan() {
        let result = serialize(&Value::NaN).unwrap();
        assert_eq!(result.json, json!("NaN"));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("number".into()))
        );
    }

    #[test]
    fn test_serialize_regexp() {
        let result = serialize(&Value::RegExp {
            source: "\\d+".into(),
            flags: "gi".into(),
        })
        .unwrap();
        assert_eq!(result.json, json!("/\\d+/gi"));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Root(TypeAnnotation::Leaf("regexp".into()))
        );
    }

    #[test]
    fn test_serialize_nested_object_with_date() {
        let mut obj = IndexMap::new();
        let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
        obj.insert("created".to_string(), Value::Date(dt));
        obj.insert("name".to_string(), Value::String("test".into()));

        let result = serialize(&Value::Object(obj)).unwrap();
        assert_eq!(
            result.json,
            json!({"created": "1970-01-01T00:00:00.000Z", "name": "test"})
        );
        let mut expected = IndexMap::new();
        expected.insert("created".to_string(), TypeAnnotation::Leaf("Date".into()));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Children(expected)
        );
    }

    #[test]
    fn test_serialize_deeply_nested_date() {
        let mut inner = IndexMap::new();
        let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
        inner.insert("date".to_string(), Value::Date(dt));
        let mut outer = IndexMap::new();
        outer.insert("meeting".to_string(), Value::Object(inner));

        let result = serialize(&Value::Object(outer)).unwrap();
        let mut expected = IndexMap::new();
        expected.insert(
            "meeting.date".to_string(),
            TypeAnnotation::Leaf("Date".into()),
        );
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Children(expected)
        );
    }

    #[test]
    fn test_serialize_array_with_mixed_types() {
        let arr = Value::Array(vec![
            Value::Number(1.0),
            Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()),
            Value::BigInt(BigInt::from(999)),
        ]);

        let result = serialize(&arr).unwrap();
        assert_eq!(result.json, json!([1.0, "1970-01-01T00:00:00.000Z", "999"]));
        let mut expected = IndexMap::new();
        expected.insert("1".to_string(), TypeAnnotation::Leaf("Date".into()));
        expected.insert("2".to_string(), TypeAnnotation::Leaf("bigint".into()));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Children(expected)
        );
    }

    #[test]
    fn test_serialize_object_with_set_containing_extended() {
        let mut obj = IndexMap::new();
        obj.insert(
            "a".to_string(),
            Value::Set(vec![Value::Number(1.0), Value::Undefined, Value::Number(2.0)]),
        );

        let result = serialize(&Value::Object(obj)).unwrap();
        assert_eq!(result.json, json!({"a": [1.0, null, 2.0]}));

        let mut inner = IndexMap::new();
        inner.insert("1".to_string(), TypeAnnotation::Leaf("undefined".into()));
        let mut expected = IndexMap::new();
        expected.insert("a".to_string(), TypeAnnotation::Node("set".into(), inner));
        assert_eq!(
            result.meta.unwrap().values.unwrap(),
            AnnotationValues::Children(expected)
        );
    }
}
