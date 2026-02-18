pub mod deserialize;
pub mod error;
pub mod path;
pub mod serialize;
pub mod value;

pub use error::{Error, Result};
pub use value::Value;

use indexmap::IndexMap;
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The superjson serialized representation, consisting of a JSON-compatible value
/// and optional metadata for type annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperJson {
    pub json: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Metadata containing type annotations and referential equality information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<AnnotationValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referential_equalities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u8>,
}

/// A type annotation node in the annotation tree.
///
/// Corresponds to JS superjson's `Tree<string>` type:
/// - `Leaf<T> = [T]` → e.g., `["Date"]`
/// - `InnerNode<T> = [T, Record<string, Tree<T>>]` → e.g., `["set", {"1": ["undefined"]}]`
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// A leaf annotation with no inner annotations: `["typeName"]`
    Leaf(String),
    /// A node annotation with inner annotations: `["typeName", {children}]`
    Node(String, IndexMap<String, TypeAnnotation>),
}

impl TypeAnnotation {
    pub fn type_name(&self) -> &str {
        match self {
            TypeAnnotation::Leaf(name) | TypeAnnotation::Node(name, _) => name,
        }
    }

    pub fn children(&self) -> Option<&IndexMap<String, TypeAnnotation>> {
        match self {
            TypeAnnotation::Leaf(_) => None,
            TypeAnnotation::Node(_, children) => Some(children),
        }
    }
}

impl Serialize for TypeAnnotation {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            TypeAnnotation::Leaf(name) => {
                let mut seq = serializer.serialize_seq(Some(1))?;
                seq.serialize_element(name)?;
                seq.end()
            }
            TypeAnnotation::Node(name, children) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(name)?;
                seq.serialize_element(children)?;
                seq.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for TypeAnnotation {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_seq(TypeAnnotationVisitor)
    }
}

struct TypeAnnotationVisitor;

impl<'de> Visitor<'de> for TypeAnnotationVisitor {
    type Value = TypeAnnotation;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a type annotation array: [\"typeName\"] or [\"typeName\", {children}]")
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> std::result::Result<TypeAnnotation, A::Error> {
        let name: String = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &"at least 1 element"))?;

        let children: Option<IndexMap<String, TypeAnnotation>> = seq.next_element()?;

        match children {
            Some(c) => Ok(TypeAnnotation::Node(name, c)),
            None => Ok(TypeAnnotation::Leaf(name)),
        }
    }
}

/// The top-level annotation values in `meta.values`.
///
/// Corresponds to JS superjson's `MinimisedTree<string>`:
/// - Root: the root value itself is annotated → `["Date"]` or `["set", {...}]`
/// - Children: child values are annotated → `{"field": ["Date"], ...}`
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationValues {
    /// Root value is an extended type.
    Root(TypeAnnotation),
    /// Children of the root value are extended types.
    Children(IndexMap<String, TypeAnnotation>),
}

impl Serialize for AnnotationValues {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            AnnotationValues::Root(ann) => ann.serialize(serializer),
            AnnotationValues::Children(map) => map.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AnnotationValues {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::Array(_) => {
                let ann: TypeAnnotation =
                    serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(AnnotationValues::Root(ann))
            }
            serde_json::Value::Object(_) => {
                let map: IndexMap<String, TypeAnnotation> =
                    serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(AnnotationValues::Children(map))
            }
            _ => Err(de::Error::custom(
                "expected array or object for annotation values",
            )),
        }
    }
}

/// Serialize a `Value` into a superjson JSON string.
///
/// This is the high-level API that produces a JSON string containing
/// both the data and type metadata.
///
/// # Examples
/// ```
/// use superjson_rs::{Value, stringify};
/// use chrono::TimeZone;
///
/// let value = Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap());
/// let json_str = stringify(&value).unwrap();
/// assert!(json_str.contains("Date"));
/// ```
pub fn stringify(value: &Value) -> Result<String> {
    let superjson = serialize::serialize(value)?;
    serde_json::to_string(&superjson).map_err(Error::from)
}

/// Parse a superjson JSON string back into a `Value`.
///
/// # Examples
/// ```
/// use superjson_rs::{Value, stringify, parse};
///
/// let value = Value::String("hello".into());
/// let json_str = stringify(&value).unwrap();
/// let parsed = parse(&json_str).unwrap();
/// assert_eq!(parsed, value);
/// ```
pub fn parse(s: &str) -> Result<Value> {
    let superjson: SuperJson = serde_json::from_str(s)?;
    deserialize::deserialize(&superjson)
}
