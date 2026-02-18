# superjson-rs

A Rust implementation of [superjson](https://github.com/blitz-js/superjson) — safely serialize and deserialize data types beyond standard JSON.

superjson-rs produces output that is **wire-compatible** with the JavaScript superjson library, so you can serialize on one side and deserialize on the other.

## Supported Types

| Type | Value variant | JS equivalent |
|------|--------------|---------------|
| null | `Null` | `null` |
| boolean | `Bool(bool)` | `true` / `false` |
| number | `Number(f64)` | `42`, `3.14` |
| string | `String(String)` | `"hello"` |
| array | `Array(Vec<Value>)` | `[1, 2, 3]` |
| object | `Object(IndexMap<String, Value>)` | `{ key: "value" }` |
| undefined | `Undefined` | `undefined` |
| Date | `Date(DateTime<Utc>)` | `new Date()` |
| BigInt | `BigInt(BigInt)` | `BigInt(42)` |
| Set | `Set(Vec<Value>)` | `new Set([1, 2])` |
| Map | `Map(Vec<(Value, Value)>)` | `new Map([["a", 1]])` |
| NaN | `NaN` | `NaN` |
| Infinity | `PosInfinity` / `NegInfinity` | `Infinity` / `-Infinity` |
| -0 | `NegZero` | `-0` |
| RegExp | `RegExp { source, flags }` | `/pattern/flags` |
| URL | `Url(String)` | `new URL("...")` |
| Error | `Error { name, message, cause }` | `new Error("...", { cause })` |

## Usage

```rust
use superjson_rs::{stringify, parse, Value};
use chrono::TimeZone;
use num_bigint::BigInt;
use indexmap::IndexMap;

// Serialize
let mut obj = IndexMap::new();
obj.insert("date".to_string(), Value::Date(chrono::Utc.timestamp_millis_opt(0).unwrap()));
obj.insert("count".to_string(), Value::BigInt(BigInt::from(42)));

let json_str = stringify(&Value::Object(obj)).unwrap();
// → {"json":{"date":"1970-01-01T00:00:00.000Z","count":"42"},"meta":{"values":{"count":["bigint"],"date":["Date"]},"v":1}}

// Deserialize
let value = parse(&json_str).unwrap();
```

## Interoperability with JS superjson

superjson-rs is designed to interoperate with the JavaScript [superjson](https://github.com/blitz-js/superjson) library. You can serialize data in JS and deserialize it in Rust, or vice versa.

```rust
// Parse output from JS superjson
let js_output = r#"{
    "json": { "date": "1970-01-01T00:00:00.000Z", "tags": [1, 2] },
    "meta": { "values": { "date": ["Date"], "tags": ["set"] }, "v": 1 }
}"#;

let value = superjson_rs::parse(js_output).unwrap();
```

## Known Limitations

- **No `referentialEqualities` support** — JS superjson's [dedupe mode](https://github.com/blitz-js/superjson#dedupe) (`new SuperJSON({ dedupe: true })`) records shared object references in `meta.referentialEqualities`. superjson-rs parses this field without error, but **does not restore the references**. Values that were deduplicated to `null` in the JSON will remain `null`. This only affects data serialized with `dedupe: true`, which is not the default in JS superjson.
- **No `Symbol` or `TypedArray` support** — These JS-specific types are not represented in the `Value` enum.
- **No custom type transformers** — JS superjson allows registering custom serializers via `registerCustom()`. This is not supported.

## License

MIT
