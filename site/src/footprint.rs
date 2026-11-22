//! The landing pages' weight, as `cargo rullst footprint --url https://rullst.win --page-weight
//! --json` measured the published pages during the deployment.
//!
//! A report is shown only when every check passes: the schema, the CLI version this site is
//! built with, the published origin and page path, a measurement from today or yesterday
//! (UTC), every resource answered with 200, nothing left unfetched and no third-party
//! origin. Otherwise the footer leaves the line out rather than show an old or partial
//! number, and a warning says why.

use std::path::Path;

use rullst::html;
use rullst::html::Markup;
use serde_json::Value;

use crate::date::Date;
use crate::pages::{Lang, RULLST_VERSION};

pub const SCHEMA: &str = "rullst.footprint.v1";
/// The published site; a report's target is this origin and the page path.
pub const ORIGIN: &str = "https://rullst.win";
/// The report of each landing, by the fixed names `--footprint` reads.
pub const FILES: [(&str, &str); 2] = [("footprint.json", "/"), ("footprint-pt-br.json", "/pt-br/")];

/// One page's measured first visit.
#[derive(Clone, Debug, PartialEq)]
pub struct Weight {
    pub bytes: u64,
    pub requests: u64,
    pub measured_on: Date,
    pub cli_version: String,
}

/// The measured weight of each landing, when its report passed the checks.
#[derive(Clone, Debug, Default)]
pub struct Weights {
    pub en: Option<Weight>,
    pub pt: Option<Weight>,
}

impl Weights {
    pub fn of(&self, lang: Lang) -> Option<&Weight> {
        if lang.is_pt() {
            self.pt.as_ref()
        } else {
            self.en.as_ref()
        }
    }
}

fn get<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |value, key| value.get(*key))
}

/// The checked first visit of `path`, or why it is not shown.
pub fn check(report: &Value, path: &str, today: Date) -> Result<Weight, String> {
    let text = |keys: &[&str]| get(report, keys).and_then(Value::as_str);
    if text(&["schema"]) != Some(SCHEMA) {
        return Err(format!("schema is not {SCHEMA}"));
    }
    let cli_version = text(&["cli_version"]).unwrap_or_default();
    if cli_version != RULLST_VERSION {
        return Err(format!(
            "measured with cargo-rullst {cli_version}, not {RULLST_VERSION}"
        ));
    }
    if !get(report, &["failure"]).is_some_and(Value::is_null) {
        return Err("the report has a failure".to_string());
    }
    let url = format!("{ORIGIN}{path}");
    if text(&["target", "mode"]) != Some("remote_url")
        || text(&["target", "url"]) != Some(url.as_str())
    {
        return Err(format!("the report is not of the published page {url}"));
    }
    let measured_on = text(&["measured_at"])
        .and_then(|at| at.get(..10))
        .and_then(Date::parse)
        .ok_or("measured_at is not a date")?;
    let age = today.days() - measured_on.days();
    if !(0..=1).contains(&age) {
        return Err(format!(
            "measured on {}, not today or yesterday ({})",
            measured_on.iso(),
            today.iso()
        ));
    }
    let weight = get(report, &["page_weight"]).ok_or("no page_weight")?;
    if text(&["page_weight", "status"]) != Some("measured")
        || text(&["page_weight", "path"]) != Some(path)
    {
        return Err(format!("the page weight of {path} was not measured"));
    }
    let count = |keys: &[&str]| get(weight, keys).and_then(Value::as_u64);
    let bytes =
        count(&["first_visit", "bytes"]).filter(|bytes| (1..=256 * 1024 * 1024).contains(bytes));
    let requests =
        count(&["first_visit", "requests"]).filter(|requests| (1..=200).contains(requests));
    let (Some(bytes), Some(requests)) = (bytes, requests) else {
        return Err("the first visit has no bytes or requests".to_string());
    };
    if count(&["unfetched"]) != Some(0) {
        return Err("some resources were not fetched".to_string());
    }
    if !get(weight, &["third_party_origins"])
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Err("the page references third-party origins".to_string());
    }
    let resources = get(weight, &["resources"])
        .and_then(Value::as_array)
        .ok_or("no resources")?;
    let first_is_page = resources
        .first()
        .is_some_and(|page| page.get("url").and_then(Value::as_str) == Some(path));
    let all_ok = resources.iter().all(|resource| {
        resource.get("status").and_then(Value::as_u64) == Some(200)
            && resource.get("error").is_some_and(Value::is_null)
    });
    if !first_is_page || !all_ok || u64::try_from(resources.len()).ok() != Some(requests) {
        return Err("a resource did not load with 200".to_string());
    }
    Ok(Weight {
        bytes,
        requests,
        measured_on,
        cli_version: cli_version.to_string(),
    })
}

fn load(file: &str, path: &str, today: Date) -> Option<Weight> {
    let checked =
        crate::input::read_json(Path::new(file)).and_then(|report| check(&report, path, today));
    match checked {
        Ok(weight) => Some(weight),
        Err(error) => {
            eprintln!("warning: page weight of {path} not shown ({error})");
            None
        }
    }
}

/// The weight of both landings, each from its report in the working directory.
pub fn load_all(today: Date) -> Weights {
    let [(en_file, en_path), (pt_file, pt_path)] = FILES;
    Weights {
        en: load(en_file, en_path, today),
        pt: load(pt_file, pt_path, today),
    }
}

/// Kilobytes (1,000 bytes), rounded, at least 1.
pub fn kilobytes(bytes: u64) -> u64 {
    (bytes.saturating_add(500) / 1000).max(1)
}

/// "This page: 484 kB in 7 requests …", for the landing's footer.
pub fn render(weight: &Weight, lang: Lang) -> Markup {
    let size = crate::benchmarks::format::number(kilobytes(weight.bytes) as f64, 0, lang);
    let requests = weight.requests;
    let day = weight.measured_on;
    let date = html! { <time datetime={day.iso()}>{day.long(lang.is_pt())}</time> };
    let tool = html! { <a href="/Rullst/book/footprint.html#a-published-site">"cargo rullst footprint "{weight.cli_version.as_str()}</a> };
    if lang.is_pt() {
        html! { "Esta página: "{size}" kB em "{requests}" requisições na primeira visita (corpos compactados, como o site publicado os serve), medida com o "{tool}" em "{date}"." }
    } else {
        html! { "This page: "{size}" kB in "{requests}" requests on a first visit (bodies compressed as the published site serves them), measured with "{tool}" on "{date}"." }
    }
}

#[cfg(test)]
mod tests {
    use super::{Date, check, kilobytes};

    fn report() -> serde_json::Value {
        serde_json::from_str(include_str!("../../.github/fixtures/site-footprint.json")).unwrap()
    }

    #[test]
    fn accepts_the_fixture_on_its_day_and_the_next() {
        let day = Date::parse("2026-11-22").unwrap();
        let weight = check(&report(), "/", day).unwrap();
        assert_eq!((weight.bytes, weight.requests), (483_633, 7));
        assert!(check(&report(), "/", Date::from_days(day.days() + 1)).is_ok());
        assert!(check(&report(), "/", Date::from_days(day.days() + 2)).is_err());
        assert!(check(&report(), "/", Date::from_days(day.days() - 1)).is_err());
        assert!(check(&report(), "/pt-br/", day).is_err());
    }

    #[test]
    fn rejects_a_failed_resource() {
        let mut report = report();
        report["page_weight"]["resources"][1]["status"] = serde_json::json!(403);
        assert!(check(&report, "/", Date::parse("2026-11-22").unwrap()).is_err());
    }

    #[test]
    fn rounds_kilobytes() {
        assert_eq!(
            (
                kilobytes(1),
                kilobytes(1499),
                kilobytes(1500),
                kilobytes(483_633)
            ),
            (1, 1, 2, 484)
        );
    }
}
