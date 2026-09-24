//! Theme tokens: every visual value the UI uses, from `ui/theme.toml` in any
//! mod, merged in load order.
//!
//! The first mod to set a token defines it; a later mod overrides it. Two
//! different mods overriding the same token is a conflict: it's reported,
//! and load order decides, exactly like def patches.

use std::collections::HashMap;
use std::path::Path;

pub type Rgba = [f32; 4];

/// Token sections and whether their values are sizes (scaled) or colours.
const SIZE_SECTIONS: &[&str] = &["space", "text", "shape"];

#[derive(Clone, Debug, Default)]
pub struct Theme {
    /// Logical pixels; multiply by `scale` for physical.
    pub space: HashMap<String, f32>,
    pub text: HashMap<String, f32>,
    pub weight: HashMap<String, u16>,
    pub color: HashMap<String, Rgba>,
    pub shape: HashMap<String, f32>,
    /// Font family; empty means the system UI font.
    pub font: String,
    /// DPI factor times the player's UI scale.
    pub scale: f32,
    pub warnings: Vec<String>,
    /// Which mod last set each token, for devtools.
    pub set_by: HashMap<String, String>,
}

pub fn parse_color(s: &str) -> Option<Rgba> {
    let h = s.strip_prefix('#')?;
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|v| v as f32 / 255.0);
    match h.len() {
        6 => Some([p(0)?, p(2)?, p(4)?, 1.0]),
        8 => Some([p(0)?, p(2)?, p(4)?, p(6)?]),
        _ => None,
    }
}

impl Theme {
    /// Merge `ui/theme.toml` from each mod directory, in load order.
    pub fn load(mods: &[(String, &Path)], scale: f32) -> Theme {
        let mut theme = Theme { scale, ..Default::default() };
        let mut defined_by: HashMap<String, String> = HashMap::new();
        let mut overridden_by: HashMap<String, String> = HashMap::new();
        for (mod_id, dir) in mods {
            let path = dir.join("ui").join("theme.toml");
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let table: toml::Table = match text.parse() {
                Ok(t) => t,
                Err(e) => {
                    theme.warnings.push(format!("{mod_id}/ui/theme.toml: {}", e.message()));
                    continue;
                }
            };
            for (section, values) in table {
                let key_base = section.clone();
                if section == "font" {
                    if let Some(f) = values.get("family").and_then(|v| v.as_str()) {
                        theme.font = f.to_string();
                        theme.record("font.family", mod_id, &mut defined_by, &mut overridden_by);
                    }
                    continue;
                }
                let Some(values) = values.as_table() else {
                    theme.warnings.push(format!("{mod_id}/ui/theme.toml: [{section}] must be a table"));
                    continue;
                };
                for (name, v) in values {
                    let key = format!("{key_base}.{name}");
                    let ok = match section.as_str() {
                        s if SIZE_SECTIONS.contains(&s) => match v.as_float().or(v.as_integer().map(|i| i as f64)) {
                            Some(n) => {
                                let map = match s {
                                    "space" => &mut theme.space,
                                    "text" => &mut theme.text,
                                    _ => &mut theme.shape,
                                };
                                map.insert(name.clone(), n as f32);
                                true
                            }
                            None => false,
                        },
                        "weight" => match v.as_integer() {
                            Some(n) => {
                                theme.weight.insert(name.clone(), n as u16);
                                true
                            }
                            None => false,
                        },
                        "color" => match v.as_str().and_then(parse_color) {
                            Some(c) => {
                                theme.color.insert(name.clone(), c);
                                true
                            }
                            None => false,
                        },
                        _ => {
                            theme.warnings.push(format!("{mod_id}/ui/theme.toml: unknown section [{section}]"));
                            break;
                        }
                    };
                    if ok {
                        theme.record(&key, mod_id, &mut defined_by, &mut overridden_by);
                    } else {
                        theme.warnings.push(format!("{mod_id}/ui/theme.toml: bad value for {key}"));
                    }
                }
            }
        }
        theme
    }

    fn record(
        &mut self,
        key: &str,
        mod_id: &str,
        defined_by: &mut HashMap<String, String>,
        overridden_by: &mut HashMap<String, String>,
    ) {
        self.set_by.insert(key.to_string(), mod_id.to_string());
        match defined_by.get(key) {
            None => {
                defined_by.insert(key.to_string(), mod_id.to_string());
            }
            Some(def) if def == mod_id => {}
            Some(_) => {
                if let Some(prev) = overridden_by.insert(key.to_string(), mod_id.to_string()) {
                    if prev != mod_id {
                        self.warnings.push(format!(
                            "theme conflict: {key} overridden by both '{prev}' and '{mod_id}' ('{mod_id}' wins by load order)"
                        ));
                    }
                }
            }
        }
    }

    /// A named size token in physical pixels, without allocating.
    pub fn size_named(&self, section: &str, name: &str) -> Result<f32, String> {
        let map = match section {
            "space" => &self.space,
            "text" => &self.text,
            _ => &self.shape,
        };
        map.get(name).map(|x| x * self.scale).ok_or_else(|| format!("unknown {section} token '{name}'"))
    }

    pub fn weight_named(&self, name: &str) -> Result<u16, String> {
        self.weight.get(name).copied().ok_or_else(|| format!("unknown weight token '{name}'"))
    }

    /// A spacing token or a number, in physical pixels.
    pub fn space(&self, v: &Token) -> Result<f32, String> {
        self.size(&self.space, "space", v)
    }

    /// A text-size token or a number, in physical pixels.
    pub fn text_size(&self, v: &Token) -> Result<f32, String> {
        self.size(&self.text, "text", v)
    }

    pub fn shape(&self, v: &Token) -> Result<f32, String> {
        self.size(&self.shape, "shape", v)
    }

    fn size(&self, map: &HashMap<String, f32>, section: &str, v: &Token) -> Result<f32, String> {
        match v {
            Token::Num(n) => Ok(n * self.scale),
            Token::Name(n) => {
                map.get(n).map(|x| x * self.scale).ok_or_else(|| format!("unknown {section} token '{n}'"))
            }
        }
    }

    pub fn weight(&self, v: &Token) -> Result<u16, String> {
        match v {
            Token::Num(n) => Ok(*n as u16),
            Token::Name(n) => self.weight.get(n).copied().ok_or_else(|| format!("unknown weight token '{n}'")),
        }
    }

    /// A colour token or a literal `#rrggbb[aa]`.
    pub fn color(&self, name: &str) -> Result<Rgba, String> {
        if name.starts_with('#') {
            return parse_color(name).ok_or_else(|| format!("bad colour '{name}'"));
        }
        self.color.get(name).copied().ok_or_else(|| format!("unknown color token '{name}'"))
    }
}

/// A token reference as written in a component: a name or a literal number.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Name(String),
    Num(f32),
}
