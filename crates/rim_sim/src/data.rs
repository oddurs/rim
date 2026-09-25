//! Script data: plain values that plugins keep in the world instead of in
//! Luau locals, so the state is hashed, saved and readable by the UI.
//!
//! Only data: nil, booleans, numbers, strings and tables of those. Tables
//! keep their keys sorted, so iteration and hashing are deterministic.

use crate::rng::mix;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Data {
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(String),
    Table(BTreeMap<Key, Data>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Key {
    Int(i64),
    Str(String),
}

/// The binary form: tagged, so it reads back without a schema.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "Data", rename = "Data")]
enum Tagged {
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(String),
    Table(BTreeMap<Key, Data>),
}

/// The text form (`rim save unpack`) is data as a script wrote it: `true`,
/// `3`, `0.5`, `"x"`, a list, or a record. A table that is neither, or a
/// record whose only key is `#`, is `{"#": [[key, value], ...]}`.
impl serde::Serialize for Data {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if !s.is_human_readable() {
            return Tagged::serialize(self, s);
        }
        match self {
            Data::Bool(b) => s.serialize_bool(*b),
            Data::Int(i) => s.serialize_i64(*i),
            Data::Num(n) if n.is_finite() => s.serialize_f64(*n),
            Data::Num(n) => Err(serde::ser::Error::custom(format!("{n} has no text form"))),
            Data::Str(v) => s.serialize_str(v),
            Data::Table(t) if is_list(t) => s.collect_seq(t.values()),
            Data::Table(t) if t.keys().all(|k| matches!(k, Key::Str(_))) && !is_pairs(t) => {
                s.collect_map(t.iter().filter_map(|(k, v)| match k {
                    Key::Str(k) => Some((k, v)),
                    Key::Int(_) => None,
                }))
            }
            Data::Table(t) => {
                let key = |k: &Key| match k {
                    Key::Int(i) => Data::Int(*i),
                    Key::Str(s) => Data::Str(s.clone()),
                };
                let pairs: Vec<(Data, &Data)> = t.iter().map(|(k, v)| (key(k), v)).collect();
                s.collect_map([("#", pairs)])
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Data {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Data, D::Error> {
        if d.is_human_readable() {
            d.deserialize_any(TextVisitor)
        } else {
            Tagged::deserialize(d)
        }
    }
}

/// Keys 1..=n, the way Luau lays out a list.
fn is_list(t: &BTreeMap<Key, Data>) -> bool {
    !t.is_empty() && t.keys().zip(1..).all(|(k, i)| *k == Key::Int(i))
}

/// A record whose only key is `#` reads as the pairs form.
fn is_pairs(t: &BTreeMap<Key, Data>) -> bool {
    t.len() == 1 && t.contains_key(&Key::Str("#".into()))
}

struct TextVisitor;

impl<'de> serde::de::Visitor<'de> for TextVisitor {
    type Value = Data;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a boolean, number, string, list or record")
    }
    fn visit_bool<E>(self, b: bool) -> Result<Data, E> {
        Ok(Data::Bool(b))
    }
    fn visit_i64<E>(self, i: i64) -> Result<Data, E> {
        Ok(Data::Int(i))
    }
    fn visit_u64<E: serde::de::Error>(self, i: u64) -> Result<Data, E> {
        i64::try_from(i).map(Data::Int).map_err(|_| E::custom(format!("{i} is too big for an integer")))
    }
    fn visit_f64<E>(self, n: f64) -> Result<Data, E> {
        Ok(Data::Num(n))
    }
    fn visit_str<E>(self, s: &str) -> Result<Data, E> {
        Ok(Data::Str(s.to_string()))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Data, A::Error> {
        let mut t = BTreeMap::new();
        while let Some(v) = seq.next_element()? {
            t.insert(Key::Int(t.len() as i64 + 1), v);
        }
        Ok(Data::Table(t))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Data, A::Error> {
        let mut t = BTreeMap::new();
        while let Some((k, v)) = map.next_entry::<String, Data>()? {
            t.insert(Key::Str(k), v);
        }
        if !is_pairs(&t) {
            return Ok(Data::Table(t));
        }
        let bad = || serde::de::Error::custom("\"#\" holds a list of [key, value] pairs");
        let Some(Data::Table(pairs)) = t.remove(&Key::Str("#".into())) else { return Err(bad()) };
        let mut out = BTreeMap::new();
        for pair in pairs.into_values() {
            let Data::Table(kv) = pair else { return Err(bad()) };
            if !is_list(&kv) {
                return Err(bad());
            }
            match kv.into_values().collect::<Vec<_>>().as_slice() {
                [Data::Int(i), v] => out.insert(Key::Int(*i), v.clone()),
                [Data::Str(s), v] => out.insert(Key::Str(s.clone()), v.clone()),
                _ => return Err(bad()),
            };
        }
        Ok(Data::Table(out))
    }
}

impl Data {
    pub fn hash(&self, h: u64) -> u64 {
        match self {
            Data::Bool(b) => mix(h ^ 1 ^ (*b as u64) << 8),
            Data::Int(i) => mix(h ^ 2 ^ mix(*i as u64)),
            Data::Num(n) => mix(h ^ 3 ^ mix(n.to_bits())),
            Data::Str(s) => s.bytes().fold(mix(h ^ 4), |h, b| mix(h ^ b as u64)),
            Data::Table(t) => t.iter().fold(mix(h ^ 5), |h, (k, v)| {
                let h = match k {
                    Key::Int(i) => mix(h ^ mix(*i as u64)),
                    Key::Str(s) => s.bytes().fold(h, |h, b| mix(h ^ b as u64)),
                };
                v.hash(h)
            }),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Data> {
        match self {
            Data::Table(t) => t.get(&Key::Str(key.to_string())),
            _ => None,
        }
    }

    /// A list entry (Luau's 1-based index).
    pub fn get_index(&self, i: i64) -> Option<&Data> {
        match self {
            Data::Table(t) => t.get(&Key::Int(i)),
            _ => None,
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            Data::Int(i) => Some(*i as f64),
            Data::Num(n) => Some(*n),
            _ => None,
        }
    }
}

/// Convert a Luau value into data. Errors name the offending path.
pub fn from_lua(v: &mlua::Value, path: &str, depth: u32) -> Result<Option<Data>, String> {
    use mlua::Value;
    if depth > 32 {
        return Err(format!("{path}: nested too deeply (a cycle?)"));
    }
    Ok(Some(match v {
        Value::Nil => return Ok(None),
        Value::Boolean(b) => Data::Bool(*b),
        Value::Integer(i) => Data::Int(*i),
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 9.0e15 {
                Data::Int(*n as i64)
            } else {
                Data::Num(*n)
            }
        }
        Value::String(s) => Data::Str(s.to_string_lossy().to_string()),
        Value::Table(t) => {
            let mut out = BTreeMap::new();
            for pair in t.clone().pairs::<Value, Value>() {
                let (k, v) = pair.map_err(|e| format!("{path}: {e}"))?;
                let key = match &k {
                    Value::Integer(i) => Key::Int(*i),
                    Value::Number(n) if n.fract() == 0.0 => Key::Int(*n as i64),
                    Value::String(s) => Key::Str(s.to_string_lossy().to_string()),
                    other => {
                        return Err(format!(
                            "{path}: table keys must be strings or integers, not {}",
                            other.type_name()
                        ))
                    }
                };
                let sub = match &key {
                    Key::Int(i) => format!("{path}[{i}]"),
                    Key::Str(s) => format!("{path}.{s}"),
                };
                if let Some(d) = from_lua(&v, &sub, depth + 1)? {
                    out.insert(key, d);
                }
            }
            Data::Table(out)
        }
        other => return Err(format!("{path}: only plain data can be stored, not a {}", other.type_name())),
    }))
}

/// Convert data into a fresh Luau value.
pub fn to_lua(lua: &mlua::Lua, d: &Data) -> mlua::Result<mlua::Value> {
    use mlua::Value;
    Ok(match d {
        Data::Bool(b) => Value::Boolean(*b),
        Data::Int(i) => Value::Integer(*i as _),
        Data::Num(n) => Value::Number(*n),
        Data::Str(s) => Value::String(lua.create_string(s)?),
        Data::Table(t) => {
            let out = lua.create_table()?;
            for (k, v) in t {
                let v = to_lua(lua, v)?;
                match k {
                    Key::Int(i) => out.raw_set(*i, v)?,
                    Key::Str(s) => out.raw_set(s.as_str(), v)?,
                }
            }
            Value::Table(out)
        }
    })
}
