//! Tests that verify the serialized JSON output matches JS superjson exactly.
//!
//! Each test includes the equivalent JS code and expected output from the
//! JavaScript superjson library.

use chrono::TimeZone;
use indexmap::IndexMap;
use num_bigint::BigInt;
use superjson_rs::{parse, stringify, Value};

/// Helper: serialize a Value and parse the resulting JSON string
/// to compare the raw JSON structure with expected JS output.
fn serialize_to_json(value: &Value) -> serde_json::Value {
    let s = stringify(value).unwrap();
    serde_json::from_str(&s).unwrap()
}

// ============================================================
// JS superjson output compatibility tests
// ============================================================

#[test]
fn js_compat_simple_date() {
    // JS: SuperJSON.serialize({ date: new Date(0) })
    // → { json: { date: "1970-01-01T00:00:00.000Z" },
    //     meta: { values: { date: ["Date"] }, v: 1 } }
    let mut obj = IndexMap::new();
    obj.insert(
        "date".to_string(),
        Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()),
    );

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "date": "1970-01-01T00:00:00.000Z" },
            "meta": { "values": { "date": ["Date"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_set_with_undefined() {
    // JS: SuperJSON.serialize({ a: new Set([1, undefined, 2]) })
    // → { json: { a: [1, null, 2] },
    //     meta: { values: { a: ["set", { "1": ["undefined"] }] }, v: 1 } }
    let mut obj = IndexMap::new();
    obj.insert(
        "a".to_string(),
        Value::Set(vec![
            Value::Number(1.0),
            Value::Undefined,
            Value::Number(2.0),
        ]),
    );

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": [1.0, null, 2.0] },
            "meta": { "values": { "a": ["set", { "1": ["undefined"] }] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_map_with_nan_key() {
    // JS: SuperJSON.serialize({ a: new Map([[NaN, null]]) })
    // → { json: { a: [["NaN", null]] },
    //     meta: { values: { a: ["map", { "0.0": ["number"] }] }, v: 1 } }
    let mut obj = IndexMap::new();
    obj.insert(
        "a".to_string(),
        Value::Map(vec![(Value::NaN, Value::Null)]),
    );

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": [["NaN", null]] },
            "meta": { "values": { "a": ["map", { "0.0": ["number"] }] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_nested_object_date() {
    // JS: SuperJSON.serialize({ meeting: { date: new Date(0) } })
    // → { json: { meeting: { date: "1970-01-01T00:00:00.000Z" } },
    //     meta: { values: { "meeting.date": ["Date"] }, v: 1 } }
    let mut inner = IndexMap::new();
    inner.insert(
        "date".to_string(),
        Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()),
    );
    let mut outer = IndexMap::new();
    outer.insert("meeting".to_string(), Value::Object(inner));

    let result = serialize_to_json(&Value::Object(outer));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "meeting": { "date": "1970-01-01T00:00:00.000Z" } },
            "meta": { "values": { "meeting.date": ["Date"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_toplevel_set() {
    // JS: SuperJSON.serialize(new Set([1, 2]))
    // → { json: [1, 2], meta: { values: ["set"], v: 1 } }
    let result = serialize_to_json(&Value::Set(vec![Value::Number(1.0), Value::Number(2.0)]));

    assert_eq!(
        result,
        serde_json::json!({
            "json": [1.0, 2.0],
            "meta": { "values": ["set"], "v": 1 }
        })
    );
}

#[test]
fn js_compat_toplevel_date() {
    // JS: SuperJSON.serialize(new Date(0))
    // → { json: "1970-01-01T00:00:00.000Z", meta: { values: ["Date"], v: 1 } }
    let result = serialize_to_json(&Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()));

    assert_eq!(
        result,
        serde_json::json!({
            "json": "1970-01-01T00:00:00.000Z",
            "meta": { "values": ["Date"], "v": 1 }
        })
    );
}

#[test]
fn js_compat_toplevel_undefined() {
    // JS: SuperJSON.serialize(undefined)
    // → { json: null, meta: { values: ["undefined"], v: 1 } }
    let result = serialize_to_json(&Value::Undefined);

    assert_eq!(
        result,
        serde_json::json!({
            "json": null,
            "meta": { "values": ["undefined"], "v": 1 }
        })
    );
}

#[test]
fn js_compat_bigint() {
    // JS: SuperJSON.serialize({ a: BigInt("1021312312412312312313") })
    // → { json: { a: "1021312312412312312313" },
    //     meta: { values: { a: ["bigint"] }, v: 1 } }
    let mut obj = IndexMap::new();
    let big: BigInt = "1021312312412312312313".parse().unwrap();
    obj.insert("a".to_string(), Value::BigInt(big));

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": "1021312312412312312313" },
            "meta": { "values": { "a": ["bigint"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_regexp() {
    // JS: SuperJSON.serialize({ a: /hello/g })
    // → { json: { a: "/hello/g" },
    //     meta: { values: { a: ["regexp"] }, v: 1 } }
    let mut obj = IndexMap::new();
    obj.insert(
        "a".to_string(),
        Value::RegExp {
            source: "hello".into(),
            flags: "g".into(),
        },
    );

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": "/hello/g" },
            "meta": { "values": { "a": ["regexp"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_special_numbers() {
    // JS: SuperJSON.serialize({ a: Infinity, b: -Infinity, c: NaN })
    let mut obj = IndexMap::new();
    obj.insert("a".to_string(), Value::PosInfinity);
    obj.insert("b".to_string(), Value::NegInfinity);
    obj.insert("c".to_string(), Value::NaN);

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": "Infinity", "b": "-Infinity", "c": "NaN" },
            "meta": { "values": { "a": ["number"], "b": ["number"], "c": ["number"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_neg_zero() {
    // JS: SuperJSON.serialize(-0)
    // → { json: "-0", meta: { values: ["number"], v: 1 } }
    let result = serialize_to_json(&Value::NegZero);

    assert_eq!(
        result,
        serde_json::json!({
            "json": "-0",
            "meta": { "values": ["number"], "v": 1 }
        })
    );
}

#[test]
fn js_compat_neg_zero_in_object() {
    // JS: SuperJSON.serialize({ a: -0 })
    let mut obj = IndexMap::new();
    obj.insert("a".to_string(), Value::NegZero);

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "a": "-0" },
            "meta": { "values": { "a": ["number"] }, "v": 1 }
        })
    );
}

#[test]
fn js_compat_no_meta_for_plain_json() {
    // JS: SuperJSON.serialize({ name: "Alice", age: 30 })
    // → { json: { name: "Alice", age: 30 } }
    // (no meta field)
    let mut obj = IndexMap::new();
    obj.insert("name".to_string(), Value::String("Alice".into()));
    obj.insert("age".to_string(), Value::Number(30.0));

    let result = serialize_to_json(&Value::Object(obj));

    assert_eq!(
        result,
        serde_json::json!({
            "json": { "name": "Alice", "age": 30.0 }
        })
    );
    // Verify no "meta" key exists
    assert!(result.get("meta").is_none());
}

// ============================================================
// Deserialize JS superjson output
// ============================================================

#[test]
fn js_compat_deserialize_js_output() {
    // Parse a JSON string that JS superjson would produce
    let js_output = r#"{
        "json": { "date": "1970-01-01T00:00:00.000Z", "name": "test" },
        "meta": { "values": { "date": ["Date"] }, "v": 1 }
    }"#;

    let value = parse(js_output).unwrap();
    let obj = value.as_object().unwrap();
    assert_eq!(
        obj.get("date").unwrap(),
        &Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap())
    );
    assert_eq!(obj.get("name").unwrap(), &Value::String("test".into()));
}

#[test]
fn js_compat_deserialize_set_with_nested_annotations() {
    let js_output = r#"{
        "json": { "a": [1, null, 2] },
        "meta": { "values": { "a": ["set", { "1": ["undefined"] }] }, "v": 1 }
    }"#;

    let value = parse(js_output).unwrap();
    let obj = value.as_object().unwrap();
    let set = obj.get("a").unwrap();
    assert_eq!(
        set,
        &Value::Set(vec![
            Value::Number(1.0),
            Value::Undefined,
            Value::Number(2.0),
        ])
    );
}

#[test]
fn js_compat_deserialize_toplevel_extended_type() {
    let js_output = r#"{
        "json": [1, 2, 3],
        "meta": { "values": ["set"], "v": 1 }
    }"#;

    let value = parse(js_output).unwrap();
    assert_eq!(
        value,
        Value::Set(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ])
    );
}
