//! Writes the https://rullst.win/ root site with Rullst's `html!` macro.
//!
//! ```text
//! rullst-site --out DIRECTORY [--benchmarks [--benchmarks-max-age-days DAYS]] [--today YYYY-MM-DD]
//! ```
//!
//! Writes the English landing, privacy and comparison pages (`/`, `/privacy.html`,
//! `/compare/`), their Brazilian Portuguese versions (`/pt-br/`, `/pt-br/privacy.html`,
//! `/pt-br/comparar/`) and the shared assets (stylesheet, script and language-switcher
//! flags) under `assets/` into DIRECTORY, which must be missing or empty. The pages also
//! reference `/Rullst/book/`, `/Rullst/images/` and `/Rullst/Rullst.png`, which this
//! repository's `pages.yml` deploys.
//!
//! `--benchmarks` reads `benchmarks.json` from the working directory: a downloaded copy
//! (at most 1 MiB) of the site summary Rullst/Benchmarks publishes (`results/site.json`,
//! schema `rullst.benchmarks.site.v2`). Its checked results fill the comparison pages'
//! Performance section and two blocks of each landing. Without the file, when it is
//! invalid or uses anything the pages do not explain, or when it was measured more than
//! `--benchmarks-max-age-days` days before `--today` (default 90), the pages keep their
//! "no current results" text and the program prints a warning instead of failing.
//!
//! The Rullst/Rullst.github.io deployment runs this program from a read-only checkout of
//! this repository's main branch; `validate-site.py` runs it too.

mod assets;
mod benchmarks;
mod date;
mod input;
mod pages;
mod verify;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use date::Date;

struct Args {
    out: PathBuf,
    benchmarks: bool,
    max_age_days: i64,
    today: Date,
}

fn parse_args() -> Result<Args, String> {
    let mut out = None;
    let mut benchmarks = false;
    let mut max_age_days = benchmarks::MAX_AGE_DAYS;
    let mut today = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--benchmarks" => benchmarks = true,
            "--benchmarks-max-age-days" => {
                max_age_days = value("--benchmarks-max-age-days")?
                    .parse()
                    .ok()
                    .filter(|days: &i64| *days >= 0)
                    .ok_or(
                        "--benchmarks-max-age-days must be a whole number of days, not negative",
                    )?;
            }
            "--today" => {
                let text = value("--today")?;
                today = Some(Date::parse(&text).ok_or("--today must be YYYY-MM-DD")?);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: rullst-site --out DIRECTORY [--benchmarks [--benchmarks-max-age-days DAYS]] \
                     [--today YYYY-MM-DD]"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        out: out.ok_or("--out is required")?,
        benchmarks,
        max_age_days,
        today: today.unwrap_or_else(Date::today),
    })
}

fn write(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn run(args: &Args) -> Result<Vec<&'static str>, String> {
    let results = if args.benchmarks {
        benchmarks::load(Path::new(benchmarks::FILE), args.today, args.max_age_days)
    } else {
        None
    };
    let assets = assets::Assets::new();
    let files = pages::render(&assets, results.as_ref());
    for (published, html) in &files {
        verify::page(published, html.as_str(), &assets)?;
    }
    if args.out.is_symlink() || (args.out.exists() && !args.out.is_dir()) {
        return Err(format!(
            "output must be a directory: {}",
            args.out.display()
        ));
    }
    let not_empty = fs::read_dir(&args.out).is_ok_and(|mut entries| entries.next().is_some());
    if not_empty {
        return Err(format!(
            "output directory is not empty: {}",
            args.out.display()
        ));
    }
    let mut published = Vec::new();
    for (path, html) in &files {
        write(&args.out.join(path), html.as_str().as_bytes())?;
        published.push(*path);
    }
    for (path, bytes) in assets.files() {
        write(&args.out.join(path), bytes)?;
        published.push(path);
    }
    Ok(published)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("rullst-site: {error}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(published) => {
            println!(
                "wrote {} files into {}: {}",
                published.len(),
                args.out.display(),
                published.join(", ")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
