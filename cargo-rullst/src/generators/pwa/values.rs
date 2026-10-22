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
    pub android: Option<AndroidValues>,
}

/// The values of `--android`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AndroidValues {
    pub host: String,
    pub package_id: String,
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
    pub android: bool,
    pub host: Option<String>,
    pub package_id: Option<String>,
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
        let android = if flags.android {
            let host = flags
                .host
                .clone()
                .ok_or_else(|| invalid("--android needs --host, the HTTPS host of the site"))?;
            validate_host(&host)?;
            let package_id = flags.package_id.clone().ok_or_else(|| {
                invalid("--android needs --package-id, an application-owned Android package name")
            })?;
            validate_package_id(&package_id)?;
            Some(AndroidValues { host, package_id })
        } else {
            None
        };
        Ok(Self {
            name,
            short_name,
            theme_color,
            background_color,
            lang,
            start_url,
            android,
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

/// A lowercase DNS name with at least two labels and an alphabetic top
/// label, without scheme, port or path: Digital Asset Links and Bubblewrap
/// need the HTTPS host the app opens.
fn validate_host(value: &str) -> io::Result<()> {
    let labels = value.split('.').collect::<Vec<_>>();
    let valid = value.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
        && labels
            .last()
            .is_some_and(|label| label.bytes().all(|byte| byte.is_ascii_lowercase()));
    if !valid {
        return Err(invalid(
            "--host must be the site's lowercase host name such as app.example.com, without https:// or a path",
        ));
    }
    Ok(())
}

/// An Android application ID: at least two dot-separated segments, each a
/// lowercase letter followed by lowercase letters, digits or underscores.
/// `com.example.*` is the placeholder namespace Google Play refuses.
fn validate_package_id(value: &str) -> io::Result<()> {
    let segments = value.split('.').collect::<Vec<_>>();
    let valid = value.len() <= 150
        && segments.len() >= 2
        && segments.iter().all(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
                && characters.all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                })
        });
    if !valid {
        return Err(invalid(
            "--package-id must be a lowercase Android package name such as com.acme.chat; each segment starts with a letter and holds letters, digits or underscores",
        ));
    }
    if value.starts_with("com.example.") {
        return Err(invalid(
            "--package-id may not use the com.example placeholder namespace; use an application-owned name",
        ));
    }
    Ok(())
}
