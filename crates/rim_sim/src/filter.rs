//! What a store takes (DESIGN.md §4f): which things, not made of which
//! materials, in what condition. Stockpile zones hold one; containers and
//! bills ask the same question, so they will too.

use crate::defs::{Category, DefDb, DefId};
use serde::{Deserialize, Serialize};

/// The whole condition range, in percent of full hp.
const WHOLE: [u8; 2] = [0, 100];

fn whole() -> [u8; 2] {
    WHOLE
}

fn is_whole(hp: &[u8; 2]) -> bool {
    *hp == WHOLE
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    /// The things it takes, sorted. An item a mod adds later is the
    /// player's to allow.
    pub allows: Vec<DefId>,
    /// Materials it refuses, sorted, whatever the thing: "no bone". A
    /// refusal list rather than an allow list, so a material a mod adds
    /// later is taken.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refuses: Vec<DefId>,
    /// The condition it takes, in percent of full hp, both ends included.
    #[serde(default = "whole", skip_serializing_if = "is_whole")]
    pub hp: [u8; 2],
}

/// One change to a filter, as a command carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterEdit {
    /// Take a thing, or stop.
    Thing { thing: DefId, on: bool },
    /// Take every item in an item category and those under it, or stop.
    Category { category: DefId, on: bool },
    /// Take things made of a material, or refuse them.
    Material { material: DefId, on: bool },
    /// Take only stacks whose condition is in this range, in percent.
    Condition { min: u8, max: u8 },
    /// Take every item there is, or none.
    All { on: bool },
}

impl Filter {
    /// Every item there is now, any material, any condition.
    pub fn everything(defs: &DefDb) -> Filter {
        let allows = (0..defs.things.len() as DefId).filter(|&d| defs.thing(d).category == Category::Item).collect();
        Filter { allows, refuses: Vec::new(), hp: WHOLE }
    }

    /// Whether it takes this thing in some material and condition.
    pub fn takes_thing(&self, def: DefId) -> bool {
        self.allows.binary_search(&def).is_ok()
    }

    /// Whether it takes a stack of `def` made of `made_of` at `hp`. A stack
    /// with no hp recorded is whole.
    pub fn takes(&self, defs: &DefDb, def: DefId, made_of: Option<DefId>, hp: Option<i32>) -> bool {
        self.takes_thing(def)
            && made_of.is_none_or(|m| self.refuses.binary_search(&m).is_err())
            && (is_whole(&self.hp) || {
                let pct = match hp {
                    Some(hp) => (hp.max(0) as i64 * 100 / defs.full_hp(def, made_of) as i64).min(100) as u8,
                    None => 100,
                };
                (self.hp[0]..=self.hp[1]).contains(&pct)
            })
    }

    /// Apply one change. Things that aren't items, materials that aren't,
    /// and categories that don't exist change nothing.
    pub fn edit(&mut self, defs: &DefDb, edit: FilterEdit) {
        let item = |d: DefId| (d as usize) < defs.things.len() && defs.thing(d).category == Category::Item;
        match edit {
            FilterEdit::Thing { thing, on } if item(thing) => set(&mut self.allows, thing, on),
            FilterEdit::Category { category, on } if (category as usize) < defs.item_categories.len() => {
                for d in defs.category_items(category) {
                    set(&mut self.allows, d, on);
                }
            }
            FilterEdit::Material { material, on } if item(material) && defs.thing(material).stuff.is_some() => {
                set(&mut self.refuses, material, !on)
            }
            FilterEdit::Condition { min, max } => {
                let (min, max) = (min.min(100), max.min(100));
                self.hp = [min.min(max), max];
            }
            FilterEdit::All { on } => {
                self.allows = if on { Filter::everything(defs).allows } else { Vec::new() };
            }
            _ => {}
        }
    }

    /// Consistent whatever a hand-edited save says: sorted, no repeats, a
    /// range that is one.
    pub fn tidy(&mut self) {
        for v in [&mut self.allows, &mut self.refuses] {
            v.sort_unstable();
            v.dedup();
        }
        let [a, b] = self.hp;
        let b = b.min(100);
        self.hp = [a.min(b), b];
    }

    pub fn hash(&self, mut h: u64) -> u64 {
        use crate::rng::mix;
        for &d in &self.allows {
            h = mix(h ^ d as u64);
        }
        // Only what's set: a filter of things alone hashes as zones did.
        if !self.refuses.is_empty() {
            h = mix(h ^ 0x5eed);
            for &d in &self.refuses {
                h = mix(h ^ d as u64);
            }
        }
        if !is_whole(&self.hp) {
            h = mix(h ^ (self.hp[0] as u64) << 8 ^ self.hp[1] as u64 ^ 0xc0de << 16);
        }
        h
    }
}

/// Put `d` in a sorted set, or take it out.
fn set(v: &mut Vec<DefId>, d: DefId, on: bool) {
    match (v.binary_search(&d), on) {
        (Err(i), true) => v.insert(i, d),
        (Ok(i), false) => {
            v.remove(i);
        }
        _ => {}
    }
}
