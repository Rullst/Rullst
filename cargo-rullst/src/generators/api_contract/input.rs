//! Strict configuration JSON: duplicate keys cannot silently replace policy.
use super::ApiError;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::{fmt, fs, io::Read, path::Path};

struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unique-key JSON")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate key"));
            }
            values.insert(key, map.next_value::<Unique>()?.0);
        }
        Ok(Unique(Value::Object(values)))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<Unique>()? {
            values.push(value.0);
        }
        Ok(Unique(Value::Array(values)))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
        Number::from_f64(v)
            .map(|n| Unique(Value::Number(n)))
            .ok_or_else(|| de::Error::custom("invalid number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
}
pub(super) fn parse(bytes: &[u8]) -> Result<Value, ApiError> {
    if bytes.len() > super::MAX_SOURCE {
        return Err(ApiError::Limit);
    }
    let value = serde_json::from_slice::<Unique>(bytes)
        .map_err(|_| ApiError::Invalid)?
        .0;
    fn budget(v: &Value, depth: usize, nodes: &mut usize) -> Result<(), ApiError> {
        *nodes += 1;
        if depth > 24 || *nodes > 8192 {
            return Err(ApiError::Limit);
        }
        match v {
            Value::Object(v) => {
                for child in v.values() {
                    budget(child, depth + 1, nodes)?;
                }
            }
            Value::Array(v) => {
                for child in v {
                    budget(child, depth + 1, nodes)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    budget(&value, 0, &mut 0)?;
    Ok(value)
}
pub(super) fn regular(path: &Path) -> Result<(), ApiError> {
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(meta) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = meta.is_symlink();
                if linked || !(meta.is_file() || meta.is_dir()) {
                    return Err(ApiError::Path);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub(super) fn read(path: &Path, max: usize) -> Result<String, ApiError> {
    regular(path)?;
    let before = fs::metadata(path)?;
    if !before.is_file() {
        return Err(ApiError::Path);
    }
    if before.len() > max as u64 {
        return Err(ApiError::Limit);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    regular(path)?;
    let after = fs::metadata(path)?;
    if bytes.len() > max
        || before.len() != after.len()
        || after.len() != bytes.len() as u64
        || before.modified()? != after.modified()?
    {
        return Err(ApiError::Limit);
    }
    String::from_utf8(bytes).map_err(|_| ApiError::Invalid)
}
