//! The `footprint` command line: flags, bounds and their validation.
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::time::Duration;

/// Bounds keep a run's memory, time and load predictable.
pub(super) const MAX_DURATION: Duration = Duration::from_secs(600);
const MIN_DURATION: Duration = Duration::from_secs(1);
pub(super) const MAX_CONCURRENCY: u16 = 256;
const MAX_PATH_BYTES: usize = 2048;

/// The validated inputs of one run.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Options {
    pub url: Option<reqwest::Url>,
    pub path: String,
    pub duration: Duration,
    pub concurrency: u16,
    pub grid_intensity: Option<f64>,
    pub cpu_watts: Option<f64>,
    pub embodied: Option<f64>,
    pub json: bool,
}

pub(crate) fn command() -> Command {
    Command::new("footprint")
        .about("Measure the app under a bounded local load: req/s, latency, CPU, memory, size, energy and an SCI estimate")
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
            Arg::new("path")
                .long("path")
                .value_name("PATH")
                .default_value("/")
                .value_parser(validate_path)
                .help("Request path for the load, such as / or /health"),
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .value_name("DURATION")
                .default_value("10s")
                .value_parser(parse_duration)
                .help("Load duration: 10s, 1m or 1500ms (1s to 10m)"),
        )
        .arg(
            Arg::new("concurrency")
                .long("concurrency")
                .value_name("N")
                .default_value("4")
                .value_parser(clap::value_parser!(u16).range(1..=i64::from(MAX_CONCURRENCY)))
                .help("Concurrent closed-loop connections (1 to 256)"),
        )
        .arg(
            Arg::new("grid-intensity")
                .long("grid-intensity")
                .value_name("gCO2e/kWh")
                .value_parser(|value: &str| non_negative(value, 10_000.0))
                .help("Grid carbon intensity from your own source; never fetched from the network"),
        )
        .arg(
            Arg::new("cpu-watts")
                .long("cpu-watts")
                .value_name("W")
                .value_parser(|value: &str| positive(value, 10_000.0))
                .help("Estimate energy as process CPU time × W when RAPL is not readable"),
        )
        .arg(
            Arg::new("embodied")
                .long("embodied")
                .value_name("gCO2e")
                .value_parser(|value: &str| non_negative(value, 1.0e9))
                .help("Embodied emissions (M) allocated to this run; otherwise not included"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Print the versioned JSON report (rullst.cli-footprint.v1)"),
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
            duration: get(matches, "duration").unwrap_or(Duration::from_secs(10)),
            concurrency: get(matches, "concurrency").unwrap_or(4),
            grid_intensity: get(matches, "grid-intensity"),
            cpu_watts: get(matches, "cpu-watts"),
            embodied: get(matches, "embodied"),
            json: get(matches, "json").unwrap_or(false),
        }
    }
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
            .try_get_matches_from(std::iter::once("footprint").chain(arguments.iter().copied()))
            .map(|matches| Options::from_matches(&matches))
    }

    #[test]
    fn defaults_and_flags_parse_into_bounded_options() {
        let defaults = parse(&[]).unwrap();
        assert_eq!(defaults.url, None);
        assert_eq!(defaults.path, "/");
        assert_eq!(defaults.duration, Duration::from_secs(10));
        assert_eq!(defaults.concurrency, 4);
        assert_eq!(
            (
                defaults.grid_intensity,
                defaults.cpu_watts,
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
            "--duration",
            "1500ms",
            "--concurrency",
            "8",
            "--grid-intensity",
            "120.5",
            "--cpu-watts",
            "15",
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
        assert_eq!(full.duration, Duration::from_millis(1500));
        assert_eq!(full.concurrency, 8);
        assert_eq!(full.grid_intensity, Some(120.5));
        assert_eq!(full.cpu_watts, Some(15.0));
        assert_eq!(full.embodied, Some(0.0));
        assert!(full.json);
    }

    #[test]
    fn out_of_range_and_malformed_values_are_usage_errors() {
        for arguments in [
            &["--duration", "0s"][..],
            &["--duration", "11m"],
            &["--duration", "999ms"],
            &["--duration", "10h"],
            &["--duration", "abc"],
            &["--concurrency", "0"],
            &["--concurrency", "257"],
            &["--grid-intensity", "-1"],
            &["--grid-intensity", "NaN"],
            &["--cpu-watts", "0"],
            &["--embodied", "inf"],
            &["--path", "health"],
            &["--path", "//evil.example/"],
            &["--path", "/a b"],
            &["--path", "/#top"],
            &["--url", "http://example.com"],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?}");
        }
        assert_eq!(parse_duration("2m"), Ok(Duration::from_secs(120)));
        assert_eq!(parse_duration(" 30 "), Ok(Duration::from_secs(30)));
    }
}
