//! The `green` command line: flags, bounds and their validation.
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::time::Duration;

/// Bounds keep a run's memory, time and load predictable.
pub(super) const MAX_DURATION: Duration = Duration::from_secs(600);
const MIN_DURATION: Duration = Duration::from_secs(1);
pub(super) const MAX_CONCURRENCY: u16 = 256;
const MAX_PATH_BYTES: usize = 2048;
const MAX_IMAGE_BYTES: usize = 255;

/// The validated inputs of one run.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Options {
    pub url: Option<reqwest::Url>,
    pub path: String,
    /// `RULLST_ENV` of the app green starts: `production` or `staging`.
    pub environment: &'static str,
    /// A local container image whose size is measured.
    pub image: Option<String>,
    /// Whether the bounded load (benchmark) runs.
    pub bench: bool,
    pub duration: Duration,
    pub concurrency: u16,
    pub cpu_watts: Option<f64>,
    pub grid_intensity: Option<f64>,
    pub embodied: Option<f64>,
    pub json: bool,
}

pub(crate) fn command() -> Command {
    Command::new("green")
        .about("(experimental) Measure binary and image size, idle memory, dependencies and benchmark CPU time; an SCI carbon estimate only from inputs you supply")
        .arg(
            Arg::new("url")
                .long("url")
                .value_name("URL")
                .value_parser(|value: &str| {
                    super::target::validate_url(value).map_err(|error| error.to_string())
                })
                .help("Measure an app already running on loopback (http://127.0.0.1:PORT); otherwise build and start the project's release binary"),
        )
        .arg(
            Arg::new("env")
                .long("env")
                .value_name("ENV")
                .default_value("production")
                .value_parser(["production", "staging"])
                .conflicts_with("url")
                .help("RULLST_ENV of the started app; staging accepts the starters' mock billing credentials"),
        )
        .arg(
            Arg::new("path")
                .long("path")
                .value_name("PATH")
                .default_value("/")
                .value_parser(validate_path)
                .help("Request path for the readiness check and the benchmark, such as / or /health"),
        )
        .arg(
            Arg::new("image")
                .long("image")
                .value_name("IMAGE")
                .value_parser(validate_image)
                .help("Also measure this local container image (docker or podman image inspect)"),
        )
        .arg(
            Arg::new("bench")
                .long("bench")
                .action(ArgAction::SetTrue)
                .help("Run a bounded closed-loop load and measure the app's CPU time per request"),
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .value_name("DURATION")
                .default_value("10s")
                .value_parser(parse_duration)
                .requires("bench")
                .help("Benchmark duration: 10s, 1m or 1500ms (1s to 10m)"),
        )
        .arg(
            Arg::new("concurrency")
                .long("concurrency")
                .value_name("N")
                .default_value("4")
                .value_parser(clap::value_parser!(u16).range(1..=i64::from(MAX_CONCURRENCY)))
                .requires("bench")
                .help("Benchmark connections, each sending one request at a time (1 to 256)"),
        )
        .arg(
            Arg::new("cpu-watts")
                .long("cpu-watts")
                .value_name("W")
                .value_parser(|value: &str| positive(value, 10_000.0))
                .requires("bench")
                .help("Your power figure per busy CPU second, used only for the SCI estimate's energy term"),
        )
        .arg(
            Arg::new("grid-intensity")
                .long("grid-intensity")
                .value_name("gCO2e/kWh")
                .value_parser(|value: &str| non_negative(value, 10_000.0))
                .requires("cpu-watts")
                .help("Grid carbon intensity from your own source, for the SCI estimate; never fetched"),
        )
        .arg(
            Arg::new("embodied")
                .long("embodied")
                .value_name("gCO2e")
                .value_parser(|value: &str| non_negative(value, 1.0e9))
                .requires("grid-intensity")
                .help("Embodied emissions (M) you allocate to this run; otherwise not included"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Print the versioned JSON report (rullst.green.v1)"),
        )
}

impl Options {
    pub(super) fn from_matches(matches: &ArgMatches) -> Self {
        fn get<T: Clone + Send + Sync + 'static>(matches: &ArgMatches, id: &str) -> Option<T> {
            matches.try_get_one::<T>(id).ok().flatten().cloned()
        }
        Self {
            url: get(matches, "url"),
            path: get(matches, "path").unwrap_or_else(|| "/".to_string()),
            environment: match get::<String>(matches, "env").as_deref() {
                Some("staging") => "staging",
                _ => "production",
            },
            image: get(matches, "image"),
            bench: get(matches, "bench").unwrap_or(false),
            duration: get(matches, "duration").unwrap_or(Duration::from_secs(10)),
            concurrency: get(matches, "concurrency").unwrap_or(4),
            cpu_watts: get(matches, "cpu-watts"),
            grid_intensity: get(matches, "grid-intensity"),
            embodied: get(matches, "embodied"),
            json: get(matches, "json").unwrap_or(false),
        }
    }
}

/// A local image reference such as `my-app:latest` or
/// `registry.example/team/app@sha256:…`: never an option (`-…`), whitespace
/// or a shell character.
pub(super) fn validate_image(value: &str) -> Result<String, String> {
    let allowed = |character: char| {
        character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/' | ':' | '@')
    };
    if value.is_empty() || value.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "the image name must have 1 to {MAX_IMAGE_BYTES} characters"
        ));
    }
    if value.starts_with('-') || !value.chars().all(allowed) {
        return Err(
            "the image name may contain letters, digits and . _ - / : @ and must not start with -"
                .to_string(),
        );
    }
    Ok(value.to_string())
}

/// `10s`, `1m`, `1500ms` or a bare number of seconds, within 1 s to 10 min.
pub(super) fn parse_duration(value: &str) -> Result<Duration, String> {
    let value = value.trim();
    let (digits, unit) = match value.find(|character: char| !character.is_ascii_digit()) {
        Some(index) => value.split_at(index),
        None => (value, "s"),
    };
    let amount: u64 = digits
        .parse()
        .map_err(|_| format!("`{value}` is not a duration such as 10s, 1m or 1500ms"))?;
    let duration = match unit {
        "ms" => Duration::from_millis(amount),
        "s" => Duration::from_secs(amount),
        "m" => Duration::from_secs(amount.saturating_mul(60)),
        _ => return Err(format!("unknown unit `{unit}`; use ms, s or m")),
    };
    if !(MIN_DURATION..=MAX_DURATION).contains(&duration) {
        return Err("the duration must be between 1s and 10m".to_string());
    }
    Ok(duration)
}

/// An origin-relative path with an optional query: never `//host`, a
/// fragment, whitespace or control characters.
pub(super) fn validate_path(value: &str) -> Result<String, String> {
    if !value.starts_with('/') || value.starts_with("//") {
        return Err("the path must start with a single `/`, such as /health".to_string());
    }
    if value.len() > MAX_PATH_BYTES {
        return Err(format!("the path is longer than {MAX_PATH_BYTES} bytes"));
    }
    if value
        .chars()
        .any(|character| character == '#' || character.is_whitespace() || character.is_control())
    {
        return Err("the path must not contain spaces, control characters or `#`".to_string());
    }
    Ok(value.to_string())
}

fn number(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("`{value}` is not a number"))
}

fn non_negative(value: &str, max: f64) -> Result<f64, String> {
    let number = number(value)?;
    if !(0.0..=max).contains(&number) {
        return Err(format!("the value must be between 0 and {max}"));
    }
    Ok(number)
}

fn positive(value: &str, max: f64) -> Result<f64, String> {
    let number = number(value)?;
    if number <= 0.0 || number > max {
        return Err(format!(
            "the value must be greater than 0 and at most {max}"
        ));
    }
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Options, clap::Error> {
        command()
            .try_get_matches_from(std::iter::once("green").chain(arguments.iter().copied()))
            .map(|matches| Options::from_matches(&matches))
    }

    #[test]
    fn defaults_and_flags_parse_into_bounded_options() {
        let defaults = parse(&[]).unwrap();
        assert_eq!(defaults.url, None);
        assert_eq!(defaults.path, "/");
        assert_eq!(defaults.environment, "production");
        assert_eq!(defaults.image, None);
        assert!(!defaults.bench);
        assert_eq!(defaults.duration, Duration::from_secs(10));
        assert_eq!(defaults.concurrency, 4);
        assert_eq!(
            (
                defaults.cpu_watts,
                defaults.grid_intensity,
                defaults.embodied
            ),
            (None, None, None)
        );
        assert!(!defaults.json);

        let full = parse(&[
            "--url",
            "http://localhost:3000",
            "--path",
            "/health?full=1",
            "--image",
            "registry.example/team/app:1.2",
            "--bench",
            "--duration",
            "1500ms",
            "--concurrency",
            "8",
            "--cpu-watts",
            "15",
            "--grid-intensity",
            "120.5",
            "--embodied",
            "0",
            "--json",
        ])
        .unwrap();
        assert_eq!(
            full.url.as_ref().map(reqwest::Url::as_str),
            Some("http://127.0.0.1:3000/")
        );
        assert_eq!(full.path, "/health?full=1");
        assert_eq!(full.image.as_deref(), Some("registry.example/team/app:1.2"));
        assert!(full.bench);
        assert_eq!(full.duration, Duration::from_millis(1500));
        assert_eq!(full.concurrency, 8);
        assert_eq!(full.cpu_watts, Some(15.0));
        assert_eq!(full.grid_intensity, Some(120.5));
        assert_eq!(full.embodied, Some(0.0));
        assert!(full.json);
        assert_eq!(parse(&["--env", "staging"]).unwrap().environment, "staging");
    }

    #[test]
    fn estimate_inputs_need_the_benchmark_and_each_other() {
        for arguments in [
            &["--duration", "5s"][..],
            &["--concurrency", "2"],
            &["--cpu-watts", "15"],
            &["--bench", "--grid-intensity", "100"],
            &["--bench", "--cpu-watts", "15", "--embodied", "1"],
        ] {
            let error = parse(arguments).unwrap_err();
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{arguments:?}"
            );
        }
        assert!(parse(&["--bench", "--cpu-watts", "15"]).is_ok());
    }

    #[test]
    fn out_of_range_and_malformed_values_are_usage_errors() {
        for arguments in [
            &["--bench", "--duration", "0s"][..],
            &["--bench", "--duration", "11m"],
            &["--bench", "--duration", "999ms"],
            &["--bench", "--duration", "10h"],
            &["--bench", "--duration", "abc"],
            &["--bench", "--concurrency", "0"],
            &["--bench", "--concurrency", "257"],
            &["--bench", "--cpu-watts", "15", "--grid-intensity", "-1"],
            &["--bench", "--cpu-watts", "15", "--grid-intensity", "NaN"],
            &["--bench", "--cpu-watts", "0"],
            &[
                "--bench",
                "--cpu-watts",
                "15",
                "--grid-intensity",
                "1",
                "--embodied",
                "inf",
            ],
            &["--path", "health"],
            &["--path", "//evil.example/"],
            &["--path", "/a b"],
            &["--path", "/#top"],
            &["--url", "http://example.com"],
            &["--env", "development"],
            &["--env", "staging", "--url", "http://127.0.0.1:3000"],
            &["--image", "--format"],
            &["--image", "app;rm"],
            &["--image", "app name"],
            &["--image", ""],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?}");
        }
        assert_eq!(parse_duration("2m"), Ok(Duration::from_secs(120)));
        assert_eq!(parse_duration(" 30 "), Ok(Duration::from_secs(30)));
        assert_eq!(
            validate_image("app@sha256:ab12"),
            Ok("app@sha256:ab12".into())
        );
        assert!(validate_image(&"a".repeat(256)).is_err());
        assert!(validate_image(&"a".repeat(255)).is_ok());
    }
}
