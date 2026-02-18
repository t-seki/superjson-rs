use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use num_bigint::BigInt;
use std::fmt;

/// A rich value type that represents all data types supported by superjson.
///
/// This extends standard JSON types with additional types like `Date`, `BigInt`,
/// `Set`, `Map`, `undefined`, special numbers, and `RegExp`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Standard JSON types
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),

    // Extended types (superjson-specific)
    Undefined,
    Date(DateTime<Utc>),
    BigInt(BigInt),
    Set(Vec<Value>),
    Map(Vec<(Value, Value)>),
    NaN,
    PosInfinity,
    NegInfinity,
    NegZero,
    RegExp { source: String, flags: String },
    Url(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Object(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{k}\": {v}")?;
                }
                write!(f, "}}")
            }
            Value::Undefined => write!(f, "undefined"),
            Value::Date(dt) => write!(f, "Date({})", dt.to_rfc3339()),
            Value::BigInt(n) => write!(f, "{n}n"),
            Value::Set(items) => {
                write!(f, "Set {{")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "}}")
            }
            Value::Map(entries) => {
                write!(f, "Map {{")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k} => {v}")?;
                }
                write!(f, "}}")
            }
            Value::NaN => write!(f, "NaN"),
            Value::PosInfinity => write!(f, "Infinity"),
            Value::NegInfinity => write!(f, "-Infinity"),
            Value::NegZero => write!(f, "-0"),
            Value::RegExp { source, flags } => write!(f, "/{source}/{flags}"),
            Value::Url(s) => write!(f, "URL({s})"),
        }
    }
}

// From trait implementations for convenient Value construction

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(f64::from(n))
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        if n.is_nan() {
            Value::NaN
        } else if n.is_infinite() {
            if n.is_sign_positive() {
                Value::PosInfinity
            } else {
                Value::NegInfinity
            }
        } else if n == 0.0 && n.is_sign_negative() {
            Value::NegZero
        } else {
            Value::Number(n)
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(arr: Vec<T>) -> Self {
        Value::Array(arr.into_iter().map(Into::into).collect())
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(dt: DateTime<Utc>) -> Self {
        Value::Date(dt)
    }
}

impl From<BigInt> for Value {
    fn from(n: BigInt) -> Self {
        Value::BigInt(n)
    }
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_undefined(&self) -> bool {
        matches!(self, Value::Undefined)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }
}
