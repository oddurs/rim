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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Int(i64),
    Str(String),
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
        Value::Integer(i) => Data::Int(*i as i64),
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
                    Value::Integer(i) => Key::Int(*i as i64),
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
