use crate::types::Estate;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn estate_hash(estate: &Estate) -> String {
    let value = serde_json::to_value(estate).unwrap_or(Value::Null);
    let canonical = canonicalize(value);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", hex_encode(&digest))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn canonicalize(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut items: Vec<(String, Value)> = map.into_iter().collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::new();
            for (k, val) in items {
                out.insert(k, canonicalize(val));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_estate_str;

    #[test]
    fn hash_is_stable() {
        let a = load_estate_str(crate::tests::example_yaml()).unwrap();
        let b = load_estate_str(crate::tests::example_yaml()).unwrap();
        assert_eq!(estate_hash(&a), estate_hash(&b));
        assert!(estate_hash(&a).starts_with("sha256:"));
    }
}
