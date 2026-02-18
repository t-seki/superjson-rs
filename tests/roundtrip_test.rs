use chrono::TimeZone;
use indexmap::IndexMap;
use num_bigint::BigInt;
use superjson_rs::{parse, stringify, Value};

/// Helper to assert that a Value survives a roundtrip through stringify/parse.
fn assert_roundtrip(value: Value) {
    let json_str = stringify(&value).expect("stringify failed");
    let parsed = parse(&json_str).expect("parse failed");
    assert_eq!(parsed, value, "roundtrip failed for: {value}");
}

#[test]
fn roundtrip_null() {
    assert_roundtrip(Value::Null);
}

#[test]
fn roundtrip_bool() {
    assert_roundtrip(Value::Bool(true));
    assert_roundtrip(Value::Bool(false));
}

#[test]
fn roundtrip_number() {
    assert_roundtrip(Value::Number(0.0));
    assert_roundtrip(Value::Number(42.5));
    assert_roundtrip(Value::Number(-100.0));
}

#[test]
fn roundtrip_string() {
    assert_roundtrip(Value::String("".into()));
    assert_roundtrip(Value::String("hello world".into()));
    assert_roundtrip(Value::String("日本語".into()));
}

#[test]
fn roundtrip_array() {
    assert_roundtrip(Value::Array(vec![]));
    assert_roundtrip(Value::Array(vec![
        Value::Number(1.0),
        Value::String("two".into()),
        Value::Bool(true),
        Value::Null,
    ]));
}

#[test]
fn roundtrip_object() {
    let mut obj = IndexMap::new();
    obj.insert("name".to_string(), Value::String("test".into()));
    obj.insert("count".to_string(), Value::Number(42.0));
    obj.insert("active".to_string(), Value::Bool(true));
    assert_roundtrip(Value::Object(obj));
}

#[test]
fn roundtrip_undefined() {
    assert_roundtrip(Value::Undefined);
}

#[test]
fn roundtrip_date() {
    let dt = chrono::Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
    assert_roundtrip(Value::Date(dt));
}

#[test]
fn roundtrip_date_epoch() {
    let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
    assert_roundtrip(Value::Date(dt));
}

#[test]
fn roundtrip_bigint() {
    assert_roundtrip(Value::BigInt(BigInt::from(0)));
    assert_roundtrip(Value::BigInt(BigInt::from(i64::MAX)));
    assert_roundtrip(Value::BigInt(BigInt::from(-42)));
}

#[test]
fn roundtrip_set() {
    assert_roundtrip(Value::Set(vec![]));
    assert_roundtrip(Value::Set(vec![
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0),
    ]));
}

#[test]
fn roundtrip_map() {
    assert_roundtrip(Value::Map(vec![]));
    assert_roundtrip(Value::Map(vec![
        (Value::String("a".into()), Value::Number(1.0)),
        (Value::String("b".into()), Value::Number(2.0)),
    ]));
}

#[test]
fn roundtrip_nan() {
    assert_roundtrip(Value::NaN);
}

#[test]
fn roundtrip_infinity() {
    assert_roundtrip(Value::PosInfinity);
    assert_roundtrip(Value::NegInfinity);
}

#[test]
fn roundtrip_neg_zero() {
    assert_roundtrip(Value::NegZero);
}

#[test]
fn roundtrip_regexp() {
    assert_roundtrip(Value::RegExp {
        source: "\\d+".into(),
        flags: "gi".into(),
    });
    assert_roundtrip(Value::RegExp {
        source: "abc".into(),
        flags: "".into(),
    });
}

#[test]
fn roundtrip_object_with_dot_in_key() {
    let mut obj = IndexMap::new();
    obj.insert(
        "a.b".to_string(),
        Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()),
    );
    assert_roundtrip(Value::Object(obj));
}

#[test]
fn roundtrip_object_with_backslash_in_key() {
    let mut obj = IndexMap::new();
    obj.insert("a\\b".to_string(), Value::Undefined);
    assert_roundtrip(Value::Object(obj));
}

#[test]
fn roundtrip_nested_object_with_dot_in_key() {
    let mut inner = IndexMap::new();
    inner.insert("x".to_string(), Value::BigInt(BigInt::from(42)));
    let mut outer = IndexMap::new();
    outer.insert("a.b".to_string(), Value::Object(inner));
    assert_roundtrip(Value::Object(outer));
}

#[test]
fn roundtrip_url() {
    assert_roundtrip(Value::Url("https://example.com/".into()));
    assert_roundtrip(Value::Url(
        "https://example.com/path?query=value&foo=bar#fragment".into(),
    ));
}

#[test]
fn roundtrip_error_simple() {
    assert_roundtrip(Value::Error {
        name: "Error".into(),
        message: "something went wrong".into(),
        cause: None,
    });
}

#[test]
fn roundtrip_error_with_string_cause() {
    assert_roundtrip(Value::Error {
        name: "Error".into(),
        message: "fail".into(),
        cause: Some(Box::new(Value::String("root cause".into()))),
    });
}

#[test]
fn roundtrip_error_with_error_cause() {
    assert_roundtrip(Value::Error {
        name: "TypeError".into(),
        message: "outer".into(),
        cause: Some(Box::new(Value::Error {
            name: "Error".into(),
            message: "inner".into(),
            cause: None,
        })),
    });
}

#[test]
fn roundtrip_error_with_date_cause() {
    let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
    assert_roundtrip(Value::Error {
        name: "Error".into(),
        message: "fail".into(),
        cause: Some(Box::new(Value::Date(dt))),
    });
}

#[test]
fn roundtrip_error_deeply_nested_cause() {
    assert_roundtrip(Value::Error {
        name: "Error".into(),
        message: "level 1".into(),
        cause: Some(Box::new(Value::Error {
            name: "Error".into(),
            message: "level 2".into(),
            cause: Some(Box::new(Value::Error {
                name: "Error".into(),
                message: "level 3".into(),
                cause: None,
            })),
        })),
    });
}

#[test]
fn roundtrip_complex_nested_structure() {
    let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
    let mut inner = IndexMap::new();
    inner.insert("date".to_string(), Value::Date(dt));
    inner.insert(
        "tags".to_string(),
        Value::Set(vec![
            Value::String("rust".into()),
            Value::String("json".into()),
        ]),
    );

    let mut obj = IndexMap::new();
    obj.insert("id".to_string(), Value::BigInt(BigInt::from(123)));
    obj.insert("data".to_string(), Value::Object(inner));
    obj.insert(
        "scores".to_string(),
        Value::Map(vec![
            (Value::String("alice".into()), Value::Number(95.0)),
            (Value::String("bob".into()), Value::NaN),
        ]),
    );
    obj.insert("deleted".to_string(), Value::Undefined);

    assert_roundtrip(Value::Object(obj));
}

#[test]
fn roundtrip_set_with_extended_types() {
    let dt = chrono::Utc.timestamp_millis_opt(0).unwrap();
    assert_roundtrip(Value::Set(vec![
        Value::Date(dt),
        Value::BigInt(BigInt::from(42)),
        Value::Undefined,
    ]));
}

#[test]
fn roundtrip_map_with_non_string_keys() {
    assert_roundtrip(Value::Map(vec![
        (Value::Number(1.0), Value::String("one".into())),
        (Value::Number(2.0), Value::String("two".into())),
    ]));
}
