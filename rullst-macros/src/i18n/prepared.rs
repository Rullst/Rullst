//! The catalog as `__translate!` reads it: each key's kind, placeholders and
//! parsed forms, built once per catalog content and shared by every `t!`
//! call of a compilation.
//!
//! `catalog!` passes only the catalog's directory, locale order and default
//! to `t!`, so a call no longer carries the whole catalog. `__translate!`
//! re-reads the files (a directory listing and a few small reads) and reuses
//! the prepared catalog while their contents are byte-for-byte the same: the
//! result depends only on the files, never on what was cached before.

use super::catalog::{self, Catalog, Message};
use super::template::{self, Segment};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Catalogs kept for one compiler or rust-analyzer process.
const CACHED_CATALOGS: usize = 8;

/// One key: its kind, placeholders and parsed forms per locale.
pub(crate) struct Entry {
    pub(crate) html: bool,
    pub(crate) plural: bool,
    /// The placeholders of every form, plus `count` for a plural message.
    pub(crate) names: BTreeSet<String>,
    /// `(category, segments)` per locale in catalog order, `other` last;
    /// single messages only have `other`.
    pub(crate) forms: Vec<Vec<(&'static str, Vec<Segment>)>>,
}

pub(crate) struct Prepared {
    pub(crate) tags: Vec<String>,
    /// The `rullst::i18n::__Plural` variant of each locale.
    pub(crate) rules: Vec<&'static str>,
    pub(crate) default: usize,
    pub(crate) entries: BTreeMap<String, Entry>,
}

impl Prepared {
    /// Prepares a validated catalog.
    pub(crate) fn new(catalog: &Catalog) -> Result<Self, String> {
        let mut entries = BTreeMap::new();
        let keys = catalog.locales.first().map(|locale| locale.messages.iter());
        for (key, message) in keys.into_iter().flatten() {
            let parse =
                |text: &str| template::parse(text).map_err(|problem| format!("`{key}` {problem}"));
            let mut forms = Vec::new();
            let mut names = BTreeSet::new();
            for locale in &catalog.locales {
                let locale_forms = match locale.messages.get(key) {
                    Some(Message::Plural(forms)) => forms
                        .iter()
                        .map(|(category, text)| Ok((*category, parse(text)?)))
                        .collect::<Result<Vec<_>, String>>()?,
                    Some(Message::Single(text)) => vec![("other", parse(text)?)],
                    None => vec![("other", Vec::new())],
                };
                for (_, segments) in &locale_forms {
                    names.extend(template::placeholders(segments));
                }
                forms.push(locale_forms);
            }
            let plural = matches!(message, Message::Plural(_));
            if plural {
                names.insert("count".to_owned());
            }
            entries.insert(
                key.clone(),
                Entry {
                    html: key.ends_with("_html"),
                    plural,
                    names,
                    forms,
                },
            );
        }
        Ok(Self {
            tags: catalog.locales.iter().map(|l| l.tag.clone()).collect(),
            rules: catalog
                .locales
                .iter()
                .map(|l| l.rule.map_or("Other", super::tag::PluralRule::variant))
                .collect(),
            default: catalog.default,
            entries,
        })
    }
}

/// What a cached catalog was built from.
#[derive(PartialEq, Eq)]
struct Key {
    root: PathBuf,
    dir: String,
    order: Vec<String>,
    default: String,
}

struct Cached {
    key: Key,
    /// `(tag, contents)` of every file, sorted by tag.
    files: Vec<(String, String)>,
    prepared: Arc<Prepared>,
}

/// The most recently prepared catalogs, oldest first.
pub(crate) struct Cache(Mutex<Vec<Cached>>);

/// The process-wide cache that `__translate!` uses.
static CACHE: Cache = Cache::new();

/// Reads `dir` (relative to `manifest_dir`) and returns its prepared
/// catalog with the locales in `order`, reusing the one already prepared
/// for identical files.
pub(crate) fn load(
    manifest_dir: &Path,
    dir: &str,
    order: &[String],
    default: &str,
) -> Result<Arc<Prepared>, Vec<String>> {
    CACHE.load(manifest_dir, dir, order, default)
}

impl Cache {
    pub(crate) const fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    /// The number of cached catalogs read from `manifest_dir`.
    #[cfg(test)]
    pub(crate) fn entries(&self, manifest_dir: &Path) -> usize {
        let cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        cache.iter().filter(|c| c.key.root == manifest_dir).count()
    }

    pub(crate) fn load(
        &self,
        manifest_dir: &Path,
        dir: &str,
        order: &[String],
        default: &str,
    ) -> Result<Arc<Prepared>, Vec<String>> {
        let sources = catalog::read_dir(manifest_dir, dir)?;
        let mut files: Vec<(String, String)> = sources
            .iter()
            .map(|source| (source.tag.clone(), source.text.clone()))
            .collect();
        files.sort();
        let key = Key {
            root: manifest_dir.to_path_buf(),
            dir: dir.to_owned(),
            order: order.to_vec(),
            default: default.to_owned(),
        };
        {
            let cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(found) = cache.iter().find(|c| c.key == key && c.files == files) {
                return Ok(Arc::clone(&found.prepared));
            }
        }
        let catalog = catalog::build(sources, default).and_then(|c| catalog::ordered(c, order))?;
        let prepared = Arc::new(Prepared::new(&catalog).map_err(|message| vec![message])?);
        let mut cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|cached| cached.key != key);
        if cache.len() >= CACHED_CATALOGS {
            cache.remove(0);
        }
        cache.push(Cached {
            key,
            files,
            prepared: Arc::clone(&prepared),
        });
        Ok(prepared)
    }
}
