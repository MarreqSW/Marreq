// Serde's generic JSON map accepts repeated keys. Reject them at every depth
// before using a document for identity checks or locating editable values.
use std::fmt;

use anyhow::Result;
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};

pub(crate) fn decode<T: DeserializeOwned>(data: &[u8]) -> Result<T> {
    // Deserialize typed objects from the original input so numeric types and
    // duplicate typed fields receive their usual serde_json validation.
    serde_json::from_slice::<Unique>(data)?;
    Ok(serde_json::from_slice(data)?)
}

pub(crate) fn value(data: &[u8]) -> Result<Value> {
    Ok(serde_json::from_slice::<Unique>(data)?.0)
}

struct Unique(Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON with unique object keys")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
        let mut object = Map::new();
        while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key {key:?}")));
            }
            object.insert(key, value);
        }
        Ok(Unique(Value::Object(object)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
        let mut array = Vec::new();
        while let Some(Unique(value)) = seq.next_element()? {
            array.push(value);
        }
        Ok(Unique(Value::Array(array)))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Unique, E> {
        Ok(Unique(Value::String(value.to_owned())))
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Unique, E> {
        Ok(Unique(Value::Bool(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(value.into())))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Unique, E> {
        Ok(Unique(Value::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Unique, E> {
        Number::from_f64(value)
            .map(|number| Unique(Value::Number(number)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
}
