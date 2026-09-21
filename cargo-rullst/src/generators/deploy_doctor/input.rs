use clap::ArgMatches;
use rullst_core::config::RullstConfig;
use std::{
    collections::BTreeMap,
    fs::{self, Metadata, OpenOptions},
    io::Read,
    path::Path,
};

const MAX_FILE: u64 = 64 * 1024;
const MAX_VALUE: usize = 8192;
pub(super) const ENV_KEYS: [&str; 3] = ["RULLST_ENV", "APP_ENV", "APP_KEY"];

pub(super) struct Snapshot {
    pub config: RullstConfig,
    pub config_present: bool,
    pub unknown_config: bool,
    pub environment: BTreeMap<String, String>,
    pub environment_selected: bool,
    pub staging: bool,
}

pub(super) enum InputError {
    File,
    Toml,
    Env,
    Process,
}
impl InputError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::File => "input_file",
            Self::Toml => "input_toml",
            Self::Env => "input_env_file",
            Self::Process => "input_process_env",
        }
    }
    pub fn guidance(&self) -> &'static str {
        match self {
            Self::File => {
                "Use a readable regular UTF-8 file at most 64 KiB, without links or reparse points; explicit files must exist."
            }
            Self::Toml => {
                "Correct the selected TOML syntax and field types; duplicate fields are rejected. Input values are withheld."
            }
            Self::Env => {
                "Use at most 512 unique KEY=value lines, with values at most 8192 bytes. Interpolation, escapes, multiline values and duplicate keys are unsupported."
            }
            Self::Process => {
                "The three selected process variables must contain bounded Unicode values; values are withheld."
            }
        }
    }
}

fn linked(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

// Resolve OS aliases in the caller's current directory once (e.g. macOS /tmp).
// Supplied descendants and final files remain link-free. Parent directories must
// be trusted against concurrent replacement; this is not a filesystem sandbox.
fn read(path: &Path, optional: bool) -> Result<Option<String>, InputError> {
    let root = fs::canonicalize(".").map_err(|_| InputError::File)?;
    let path = root.join(path);
    for ancestor in path.ancestors() {
        if ancestor == path {
            continue;
        }
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| InputError::File)?;
        if linked(&metadata) || !metadata.is_dir() {
            return Err(InputError::File);
        }
    }
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(InputError::File),
    };
    if !metadata.is_file() || linked(&metadata) || metadata.len() > MAX_FILE {
        return Err(InputError::File);
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(&path).map_err(|_| InputError::File)?;
    let opened = file.metadata().map_err(|_| InputError::File)?;
    if !opened.is_file() || linked(&opened) || opened.len() != metadata.len() {
        return Err(InputError::File);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| InputError::File)?;
    if bytes.len() as u64 > MAX_FILE || bytes.len() as u64 != opened.len() {
        return Err(InputError::File);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| InputError::File)
}

fn env_file(contents: &str) -> Result<BTreeMap<String, String>, InputError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut values = BTreeMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Avoid dotenvy's process-variable substitution and implicit multiline
        // semantics. Parse only this documented literal single-line profile.
        if line.contains(['$', '\\', '\0']) {
            return Err(InputError::Env);
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let (key, value) = line.split_once('=').ok_or(InputError::Env)?;
        let key = key.trim();
        if key.is_empty()
            || key.len() > 128
            || !key
                .bytes()
                .enumerate()
                .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
            || !seen.insert(key)
            || seen.len() > 512
        {
            return Err(InputError::Env);
        }
        let value = value.trim();
        let value = if value.starts_with(['\'', '"']) {
            let quote = value.as_bytes()[0] as char;
            let rest = &value[1..];
            let end = rest.find(quote).ok_or(InputError::Env)?;
            let tail = &rest[end + 1..];
            if !tail.is_empty()
                && !(tail.starts_with([' ', '\t']) && tail.trim_start().starts_with('#'))
            {
                return Err(InputError::Env);
            }
            &rest[..end]
        } else {
            let (literal, tail) = value
                .find([' ', '\t'])
                .map_or((value, ""), |at| value.split_at(at));
            if !tail.is_empty() && !tail.trim_start().starts_with('#') {
                return Err(InputError::Env);
            }
            if literal.contains(['\'', '"']) || literal.chars().any(char::is_whitespace) {
                return Err(InputError::Env);
            }
            literal
        };
        if value.len() > MAX_VALUE || value.chars().any(char::is_control) {
            return Err(InputError::Env);
        }
        if ENV_KEYS.contains(&key) {
            values.insert(key.to_owned(), value.to_owned());
        }
    }
    Ok(values)
}

fn has_unknown_config(value: &toml::Table) -> bool {
    let known: &[(&str, &[&str])] = &[
        ("app", &["env", "port"]),
        ("database", &["url"]),
        ("storage", &["root"]),
        (
            "security",
            &[
                "csrf_same_site",
                "cors_allow_origins",
                "cors_allow_credentials",
                "csp",
                "coep",
                "user_agent_blocklist",
                "enable_pii_masking",
                "csrf_signed_webhook_paths",
            ],
        ),
    ];
    value.iter().any(|(section, value)| {
        let Some((_, keys)) = known.iter().find(|(name, _)| *name == section) else {
            return true;
        };
        value
            .as_table()
            .is_none_or(|table| table.keys().any(|key| !keys.contains(&key.as_str())))
    })
}

pub(super) fn load(matches: &ArgMatches) -> Result<Snapshot, InputError> {
    let explicit = matches.get_one::<String>("config");
    let content = read(
        Path::new(explicit.map_or("Rullst.toml", String::as_str)),
        explicit.is_none(),
    )?;
    let document: toml::Table =
        toml::from_str(content.as_deref().unwrap_or("")).map_err(|_| InputError::Toml)?;
    let config =
        RullstConfig::from_toml(content.as_deref().unwrap_or("")).map_err(|_| InputError::Toml)?;
    let env_path = matches.get_one::<String>("env-file");
    let environment = if let Some(path) = env_path {
        env_file(&read(Path::new(path), false)?.ok_or(InputError::File)?)?
    } else if matches.get_flag("process-env") {
        let mut env = BTreeMap::new();
        for key in ENV_KEYS {
            match std::env::var(key) {
                Ok(value) if value.len() <= MAX_VALUE => {
                    env.insert(key.to_owned(), value);
                }
                Err(std::env::VarError::NotPresent) => {}
                _ => return Err(InputError::Process),
            }
        }
        env
    } else {
        BTreeMap::new()
    };
    Ok(Snapshot {
        config,
        config_present: content.is_some(),
        unknown_config: has_unknown_config(&document),
        environment,
        environment_selected: env_path.is_some() || matches.get_flag("process-env"),
        staging: matches
            .get_one::<String>("target")
            .is_some_and(|target| target == "staging"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_literal_profile_matches_the_runtime_dotenv_parser() {
        for source in [
            "RULLST_ENV=production\nAPP_KEY=key#literal\n",
            "export RULLST_ENV='staging' # comment\r\nAPP_ENV=production\r\n",
            "APP_KEY=\"space in literal and # hash\"\nAPP_ENV=prod # comment\n",
            "APP_KEY=literal==padding\nRULLST_ENV=\nAPP_ENV=production\n",
        ] {
            let ours = env_file(source).unwrap_or_else(|_| panic!("supported fixture rejected"));
            let runtime: BTreeMap<String, String> = dotenvy::from_read_iter(source.as_bytes())
                .map(Result::unwrap)
                .collect();
            assert_eq!(ours, runtime);
        }
        let hash_suffix =
            env_file("RULLST_ENV=production#suffix").unwrap_or_else(|_| panic!("literal hash"));
        assert_eq!(
            hash_suffix.get("RULLST_ENV").map(String::as_str),
            Some("production#suffix")
        );
    }

    #[test]
    fn exact_value_and_assignment_limits_are_accepted_before_excess_is_rejected() {
        let value = format!("APP_KEY={}\n", "a".repeat(MAX_VALUE));
        assert!(env_file(&value).is_ok());
        assert!(env_file(&(value.trim_end().to_owned() + "a\n")).is_err());
        let entries: String = (0..512).map(|i| format!("KEY_{i}=value\n")).collect();
        assert!(env_file(&entries).is_ok());
        assert!(env_file(&(entries + "KEY_512=value\n")).is_err());
    }
}
