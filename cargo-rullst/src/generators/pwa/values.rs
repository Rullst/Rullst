//! The application values `make:pwa` writes into the web manifest and the
//! Bubblewrap configuration, validated before any file is planned.
//!
//! The name rules come from the retired Omni identity step, so a name that
//! Omni accepted is still accepted here.
use std::io;

/// The theme colour when `--theme-color` is absent (Tailwind slate-900).
pub(super) const DEFAULT_THEME_COLOR: &str = "#0f172a";
/// The background colour when `--background-color` is absent (slate-950,
/// the canvas of the placeholder icons).
pub(super) const DEFAULT_BACKGROUND_COLOR: &str = "#020617";

/// Validated values shared by every generated file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PwaValues {
    pub name: String,
    pub short_name: String,
    pub theme_color: String,
    pub background_color: String,
    pub lang: String,
    pub start_url: String,
}

/// The flags as given; `None` takes the default.
#[derive(Debug, Clone, Default)]
pub(super) struct PwaFlags {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub theme_color: Option<String>,
    pub background_color: Option<String>,
    pub lang: Option<String>,
    pub start_url: Option<String>,
}

pub(super) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

impl PwaValues {
    /// Validates `flags`; `package_name` is the Cargo package name that the
    /// default display name is derived from.
    pub fn resolve(flags: &PwaFlags, package_name: &str) -> io::Result<Self> {
        let name = flags
            .name
            .clone()
            .unwrap_or_else(|| humanize_package_name(package_name));
        validate_name("--name", &name)?;
        let short_name = match &flags.short_name {
            Some(short_name) => short_name.clone(),
            None => default_short_name(&name),
        };
        validate_name("--short-name", &short_name)?;
        let theme_color = color("--theme-color", flags.theme_color.as_deref())?
            .unwrap_or_else(|| DEFAULT_THEME_COLOR.to_owned());
        let background_color = color("--background-color", flags.background_color.as_deref())?
            .unwrap_or_else(|| DEFAULT_BACKGROUND_COLOR.to_owned());
        let lang = flags.lang.clone().unwrap_or_else(|| "en".to_owned());
        validate_lang(&lang)?;
        let start_url = flags.start_url.clone().unwrap_or_else(|| "/".to_owned());
        validate_start_url(&start_url)?;
        Ok(Self {
            name,
            short_name,
            theme_color,
            background_color,
            lang,
            start_url,
        })
    }
}

/// `acme-chat` -> `Acme Chat`.
pub(super) fn humanize_package_name(package_name: &str) -> String {
    package_name
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The name itself when it has at most 12 characters (what launchers show
/// without truncating), else its first word cut to 12 characters.
fn default_short_name(name: &str) -> String {
    if name.chars().count() <= 12 {
        return name.to_owned();
    }
    let first = name.split(' ').next().unwrap_or(name);
    first.chars().take(12).collect()
}

fn validate_name(flag: &str, value: &str) -> io::Result<()> {
    let length = value.chars().count();
    let has_invalid_character = value.chars().any(|character| {
        !(character.is_alphanumeric() || character == ' ' || matches!(character, '-' | '_' | '.'))
    });
    if !(1..=64).contains(&length)
        || value.trim() != value
        || value.starts_with('.')
        || has_invalid_character
    {
        return Err(invalid(format!(
            "{flag} must contain 1-64 letters, numbers, spaces, dots, hyphens or underscores and may not start with a dot"
        )));
    }
    Ok(())
}

/// `#RRGGBB`, returned in lower case.
fn color(flag: &str, value: Option<&str>) -> io::Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let digits = value.strip_prefix('#').unwrap_or("");
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid(format!(
            "{flag} must be a #RRGGBB colour such as #0f172a"
        )));
    }
    Ok(Some(value.to_ascii_lowercase()))
}

/// A BCP 47 tag of the common shape: a 2-3 letter language and optional
/// 2-8 character subtags (`en`, `pt-BR`, `zh-Hant`).
fn validate_lang(value: &str) -> io::Result<()> {
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or("");
    let valid = (2..=3).contains(&language.len())
        && language.bytes().all(|byte| byte.is_ascii_alphabetic())
        && parts.all(|part| {
            (2..=8).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    if !valid {
        return Err(invalid("--lang must be a language tag such as en or pt-BR"));
    }
    Ok(())
}

/// A same-origin path: it starts with one `/` and holds no space, quote,
/// backslash or control character.
fn validate_start_url(value: &str) -> io::Result<()> {
    let valid = value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 512
        && !value.chars().any(|character| {
            character.is_control() || matches!(character, ' ' | '"' | '\'' | '\\' | '<' | '>')
        });
    if !valid {
        return Err(invalid(
            "--start-url must be a path on this site such as / or /?source=pwa",
        ));
    }
    Ok(())
}
