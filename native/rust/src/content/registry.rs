//! The registry of tags, traits and statuses (Decision 128), `content/registry/*.json`: the one
//! place each is defined, described and tagged. A trait's tags say where it may sit (each tag's
//! targets: unit, leader, weapon, armour, tool, material, structure); it may sit where all its
//! tags agree, narrowed by its "only". A trait with a "default" is implicit: every unit has it
//! at that level without listing it (climber 0, swimmer 0; spec 30). The loader holds the
//! content to it (`checks.rs`); the designer's Library browses the same files
//! (`designer/registry.rs`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{load_file, LoadError};

/// The registry's directory under `content/`.
pub const REGISTRY: &str = "registry";

/// The three files' lists.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Registry {
    pub tags: Vec<Tag>,
    pub traits: Vec<Trait>,
    pub statuses: Vec<Status>,
}

/// A tag: what kind of thing it is put on (`tags`: trait, unit, item, status), and, for a
/// trait's tag, the kinds of thing its traits may sit on.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub tags: String,
    pub targets: Vec<String>,
    pub description: String,
}

/// A rated trait (Decision 64): its tags, its "only" if it has one, and its implicit level.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Trait {
    pub id: String,
    pub tags: Vec<String>,
    pub only: Option<Vec<String>>,
    pub default: Option<i64>,
    pub description: String,
}

/// A status a unit may have (Decision 125), with its tags.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Status {
    pub id: String,
    pub tags: Vec<String>,
    pub description: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TagsFile {
    #[serde(default, rename = "about")]
    _about: String,
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TraitsFile {
    #[serde(default, rename = "about")]
    _about: String,
    traits: Vec<Trait>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusesFile {
    #[serde(default, rename = "about")]
    _about: String,
    statuses: Vec<Status>,
}

impl Registry {
    /// The three files under `root` (the repo's `content/`).
    pub fn load(root: &Path) -> Result<Registry, LoadError> {
        let dir = root.join(REGISTRY);
        let tags: TagsFile = load_file(&dir.join("tags.json"))?;
        let traits: TraitsFile = load_file(&dir.join("traits.json"))?;
        let statuses: StatusesFile = load_file(&dir.join("statuses.json"))?;
        Ok(Registry { tags: tags.tags, traits: traits.traits, statuses: statuses.statuses })
    }

    pub fn tag(&self, id: &str) -> Option<&Tag> {
        self.tags.iter().find(|tag| tag.id == id)
    }

    pub fn trait_entry(&self, id: &str) -> Option<&Trait> {
        self.traits.iter().find(|entry| entry.id == id)
    }

    /// The kinds of thing the trait may sit on: where all its tags' targets agree, within its
    /// "only" if it has one. Empty for an unknown trait or one without tags.
    pub fn targets(&self, trait_id: &str) -> Vec<String> {
        let Some(entry) = self.trait_entry(trait_id) else {
            return Vec::new();
        };
        let mut out: Option<Vec<String>> = None;
        for tag in &entry.tags {
            let reach = self.tag(tag).map(|tag| tag.targets.clone()).unwrap_or_default();
            out = Some(match out {
                None => reach,
                Some(kept) => kept.into_iter().filter(|kind| reach.contains(kind)).collect(),
            });
        }
        let mut out = out.unwrap_or_default();
        if let Some(only) = &entry.only {
            out.retain(|kind| only.contains(kind));
        }
        out
    }

    /// The implicit traits, id -> the level every unit has without listing them.
    pub fn implicit(&self) -> BTreeMap<String, i64> {
        self.traits.iter().filter_map(|entry| Some((entry.id.clone(), entry.default?))).collect()
    }

    /// What is wrong with `traits` on a thing of `kinds` (the first is named in the message),
    /// as (the trait's id, the message): an unknown trait, or one that may not sit on any of
    /// those kinds.
    pub fn trait_problems(
        &self,
        traits: &BTreeMap<String, i64>,
        kinds: &[&str],
    ) -> Option<(String, String)> {
        for trait_id in traits.keys() {
            if self.trait_entry(trait_id).is_none() {
                return Some((trait_id.clone(), format!("unknown trait '{trait_id}'")));
            }
            if !self.targets(trait_id).iter().any(|kind| kinds.contains(&kind.as_str())) {
                let message = format!("trait '{trait_id}' can't sit on a {}", kinds[0]);
                return Some((trait_id.clone(), message));
            }
        }
        None
    }

    /// What is wrong with `tags` on a thing of `tagged` (unit, item, trait, status), as (the
    /// tag's index, the message): an unknown tag, or one not for that kind of thing.
    pub fn tag_problems(&self, tags: &[String], tagged: &str) -> Option<(usize, String)> {
        for (index, id) in tags.iter().enumerate() {
            match self.tag(id) {
                None => return Some((index, format!("unknown tag '{id}'"))),
                Some(tag) if tag.tags != tagged => {
                    let message = format!("tag '{id}' is for a {}, not a {tagged}", tag.tags);
                    return Some((index, message));
                }
                Some(_) => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::Registry;

    /// How the code reads a trait by name: the crate's rules, and GDScript's map definitions.
    const READS: [&str; 3] = ["traits.get(", "trait_level(", "_marches(squad,"];

    /// Every trait the code reads by name (from `trait_level("climber")` and the like), with a
    /// file that reads it.
    fn traits_read(dir: &Path, found: &mut Vec<(String, PathBuf)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                traits_read(&path, found);
                continue;
            }
            if !path.extension().is_some_and(|ext| ext == "rs" || ext == "gd") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for read in READS {
                for (at, _) in text.match_indices(read) {
                    let rest = text[at + read.len()..].trim_start();
                    let Some(quoted) = rest.strip_prefix('"') else { continue };
                    let name: String = quoted
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase() || *c == '_')
                        .collect();
                    if quoted[name.len()..].starts_with('"') && !name.is_empty() {
                        found.push((name, path.clone()));
                    }
                }
            }
        }
    }

    #[test]
    fn every_trait_the_code_reads_is_registered() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let registry = Registry::load(&repo.join("content")).unwrap();
        let mut found = Vec::new();
        traits_read(&repo.join("native/rust/src"), &mut found);
        traits_read(&repo.join("content/definitions"), &mut found);
        let mut names: Vec<&String> = found.iter().map(|(name, _)| name).collect();
        names.sort();
        names.dedup();
        assert!(names.len() > 5, "the scan found only {names:?}");
        for (name, path) in &found {
            assert!(
                registry.trait_entry(name).is_some(),
                "{} reads trait '{name}', which isn't registered",
                path.display()
            );
        }
    }
}
