//! Reads `<dir>/<tag>.toml` catalogs and validates them against each other.

use super::tag::{self, PLURAL_LANGUAGES, PluralRule};
use super::template;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Largest catalog file read, so a stray file cannot stall the build.
const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// Most locale files in one catalog.
const MAX_LOCALES: usize = 64;
/// Errors reported before the rest are summarized.
const MAX_ERRORS: usize = 30;
const CATEGORIES: [&str; 6] = ["zero", "one", "two", "few", "many", "other"];

/// A catalog file's contents before parsing.
pub(crate) struct Source {
    pub(crate) tag: String,
    /// Path shown in errors, relative to the crate (`locales/en.toml`).
    pub(crate) file: String,
    /// Absolute path, tracked with `include_bytes!`.
    pub(crate) path: PathBuf,
    pub(crate) text: String,
}

/// A message in one locale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    Single(String),
    /// Templates by CLDR category, in CLDR order (`other` last).
    Plural(Vec<(&'static str, String)>),
}

impl Message {
    fn templates(&self) -> impl Iterator<Item = &String> {
        let (single, forms) = match self {
            Self::Single(text) => (Some(text), &[][..]),
            Self::Plural(forms) => (None, &forms[..]),
        };
        single.into_iter().chain(forms.iter().map(|(_, text)| text))
    }
}

pub(crate) struct LocaleCatalog {
    pub(crate) tag: String,
    pub(crate) file: String,
    pub(crate) path: PathBuf,
    pub(crate) rule: Option<PluralRule>,
    pub(crate) messages: BTreeMap<String, Message>,
}

/// A validated catalog: every locale has the same keys, shapes and
/// placeholders. Locales are sorted by tag.
pub(crate) struct Catalog {
    pub(crate) locales: Vec<LocaleCatalog>,
    pub(crate) default: usize,
}

/// Collects error messages up to [`MAX_ERRORS`].
#[derive(Default)]
pub(crate) struct Errors(Vec<String>, usize);

impl Errors {
    pub(crate) fn push(&mut self, message: String) {
        if self.0.len() < MAX_ERRORS {
            self.0.push(message);
        } else {
            self.1 += 1;
        }
    }

    pub(crate) fn into_result<T>(self, value: T) -> Result<T, Vec<String>> {
        let Self(mut messages, more) = self;
        if more > 0 {
            messages.push(format!("{more} more catalog errors not shown"));
        }
        if messages.is_empty() {
            Ok(value)
        } else {
            Err(messages)
        }
    }
}

/// Reads every `*.toml` file of `dir` (relative to `manifest_dir`).
pub(crate) fn read_dir(manifest_dir: &Path, dir: &str) -> Result<Vec<Source>, Vec<String>> {
    let root = manifest_dir.join(dir);
    let entries = std::fs::read_dir(&root).map_err(|error| {
        vec![format!(
            "cannot read the catalog directory `{dir}`: {error}"
        )]
    })?;
    let mut sources = Vec::new();
    let mut errors = Errors::default();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(tag) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let file = format!("{}/{tag}.toml", dir.trim_end_matches('/'));
        match std::fs::metadata(&path) {
            Ok(meta) if meta.len() > MAX_FILE_BYTES => {
                errors.push(format!("{file}: larger than 1 MiB"));
                continue;
            }
            Ok(_) => {}
            Err(error) => {
                errors.push(format!("{file}: {error}"));
                continue;
            }
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => sources.push(Source {
                tag: tag.to_owned(),
                file,
                path,
                text,
            }),
            Err(error) => errors.push(format!("{file}: {error}")),
        }
    }
    if sources.is_empty() && errors.0.is_empty() {
        errors.push(format!("no `*.toml` catalogs in `{dir}`"));
    }
    if sources.len() > MAX_LOCALES {
        errors.push(format!("`{dir}` has more than {MAX_LOCALES} catalogs"));
    }
    errors.into_result(sources)
}

/// Parses and cross-checks catalog sources.
pub(crate) fn build(mut sources: Vec<Source>, default: &str) -> Result<Catalog, Vec<String>> {
    sources.sort_by(|a, b| a.tag.cmp(&b.tag));
    let mut errors = Errors::default();
    let mut locales = Vec::new();
    for source in sources {
        if let Err(message) = tag::check(&source.tag) {
            errors.push(format!("{}: {message}", source.file));
            continue;
        }
        let table = match source.text.parse::<toml::Table>() {
            Ok(table) => table,
            Err(error) => {
                errors.push(format!("{}: invalid TOML: {error}", source.file));
                continue;
            }
        };
        let mut messages = BTreeMap::new();
        flatten(&source.file, "", &table, &mut messages, &mut errors);
        locales.push(LocaleCatalog {
            rule: PluralRule::for_tag(&source.tag),
            tag: source.tag,
            file: source.file,
            path: source.path,
            messages,
        });
    }
    let found: Vec<&str> = locales.iter().map(|l| l.tag.as_str()).collect();
    let default_index = found.iter().position(|tag| *tag == default);
    if default_index.is_none() && !found.is_empty() {
        errors.push(format!(
            "the default locale `{default}` has no catalog file (found: {})",
            found.join(", ")
        ));
    }
    validate(&locales, &mut errors);
    errors.into_result(Catalog {
        locales,
        default: default_index.unwrap_or_default(),
    })
}

fn flatten(
    file: &str,
    prefix: &str,
    table: &toml::Table,
    out: &mut BTreeMap<String, Message>,
    errors: &mut Errors,
) {
    for (segment, value) in table {
        let valid = !segment.is_empty()
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        if !valid {
            errors.push(format!(
                "{file}: key `{segment}` may use only ASCII letters, digits, `_` and `-`"
            ));
            continue;
        }
        let key = if prefix.is_empty() {
            segment.clone()
        } else {
            format!("{prefix}.{segment}")
        };
        match value {
            toml::Value::String(text) => {
                out.insert(key, Message::Single(text.clone()));
            }
            toml::Value::Table(inner) if is_plural(inner) => {
                out.insert(key, plural(inner));
            }
            toml::Value::Table(inner) => flatten(file, &key, inner, out, errors),
            _ => errors.push(format!("{file}: `{key}` must be a string or a table")),
        }
    }
}

/// A table of plural categories with an `other` form is a plural message.
fn is_plural(table: &toml::Table) -> bool {
    table.contains_key("other")
        && table
            .iter()
            .all(|(key, value)| CATEGORIES.contains(&key.as_str()) && value.is_str())
}

/// The forms of a table that [`is_plural`] accepted, in CLDR order.
fn plural(table: &toml::Table) -> Message {
    let forms = CATEGORIES
        .iter()
        .filter_map(|category| {
            let text = table.get(*category)?.as_str()?;
            Some((*category, text.to_owned()))
        })
        .collect();
    Message::Plural(forms)
}

/// Checks that keys, message shapes and placeholders agree in every locale.
fn validate(locales: &[LocaleCatalog], errors: &mut Errors) {
    let keys: BTreeSet<&String> = locales.iter().flat_map(|l| l.messages.keys()).collect();
    for key in keys {
        let mut reference: Option<(&LocaleCatalog, &Message, BTreeSet<String>)> = None;
        for locale in locales {
            let Some(message) = locale.messages.get(key) else {
                let defined = locales.iter().find(|l| l.messages.contains_key(key));
                let defined = defined.map_or("another catalog", |l| l.file.as_str());
                errors.push(format!(
                    "{}: missing key `{key}` (defined in {defined})",
                    locale.file
                ));
                continue;
            };
            let Some(names) = placeholders(locale, key, message, errors) else {
                continue;
            };
            check_plural(locale, key, message, errors);
            let Some((first, first_message, first_names)) = &reference else {
                reference = Some((locale, message, names));
                continue;
            };
            let plural = matches!(message, Message::Plural(_));
            if plural != matches!(first_message, Message::Plural(_)) {
                let (plural_file, single_file) = if plural {
                    (&locale.file, &first.file)
                } else {
                    (&first.file, &locale.file)
                };
                errors.push(format!(
                    "`{key}` is a plural message in {plural_file} but a single message in {single_file}"
                ));
            } else if &names != first_names {
                errors.push(format!(
                    "`{key}` has different placeholders: {} uses {} but {} uses {}",
                    first.file,
                    list(first_names),
                    locale.file,
                    list(&names)
                ));
            }
        }
    }
}

/// The placeholder names of a message, or `None` after reporting a
/// malformed template.
fn placeholders(
    locale: &LocaleCatalog,
    key: &str,
    message: &Message,
    errors: &mut Errors,
) -> Option<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for text in message.templates() {
        match template::parse(text) {
            Ok(segments) => names.extend(template::placeholders(&segments)),
            Err(problem) => {
                errors.push(format!("{}: `{key}` {problem}", locale.file));
                return None;
            }
        }
    }
    Some(names)
}

/// Checks that a plural message defines exactly the forms its locale's
/// rule selects (`many` may be left out where it only means millions).
fn check_plural(locale: &LocaleCatalog, key: &str, message: &Message, errors: &mut Errors) {
    let Message::Plural(forms) = message else {
        return;
    };
    let (file, tag) = (&locale.file, &locale.tag);
    let Some(rule) = locale.rule else {
        errors.push(format!(
            "{file}: `{key}` is a plural message, but Rullst has no plural rule for `{tag}` \
             (supported: {PLURAL_LANGUAGES}); use separate keys instead"
        ));
        return;
    };
    let missing: Vec<&str> = rule
        .required()
        .iter()
        .copied()
        .filter(|category| !forms.iter().any(|(defined, _)| defined == category))
        .collect();
    if !missing.is_empty() {
        let needs = match &missing[..] {
            [one] => format!("a `{one}` form"),
            _ => format!("{} forms", and_list(&missing)),
        };
        errors.push(format!(
            "{file}: plural message `{key}` needs {needs} for `{tag}`, which uses {}",
            and_list(rule.required())
        ));
    }
    for (category, _) in forms.iter().filter(|(category, _)| !rule.allows(category)) {
        errors.push(format!(
            "{file}: `{tag}` has no `{category}` plural form; remove `{category}` from `{key}`"
        ));
    }
}

/// "`a`", "`a` and `b`", "`a`, `b` and `c`".
fn and_list(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("`{item}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}

fn list(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "no placeholders".into();
    }
    let names: Vec<String> = names.iter().map(|name| format!("{{{name}}}")).collect();
    names.join(", ")
}
