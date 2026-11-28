//! The checks every value passes before a page shows it; an error discards the whole file.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::{
    CATEGORIES, Cell, FLAGS, MEMORY, MINIMAL, Results, Row, SCHEMA, STABILITY, STATUSES, WEIGHTS,
    known_rows, row_name,
};
use crate::date::Date;
use crate::input::{show, truthy};

const NOTE_LIMIT: usize = 1000;
const READING_LIMIT: usize = 4000;
const RUN_PREFIX: &str = "https://github.com/Rullst/Benchmarks/actions/runs/";

type Checked<T> = Result<T, String>;

fn known(list: &[&'static str], value: Option<&Value>) -> Option<&'static str> {
    let text = value?.as_str()?;
    list.iter().copied().find(|item| *item == text)
}

fn metric(name: &str, key: &str, raw: &Value, limit: f64) -> Checked<f64> {
    let value = match raw {
        Value::Number(number) => number.as_f64(),
        _ => None,
    }
    .ok_or(format!("{name}: {key} is not a number"))?;
    if !value.is_finite() || !(0.0..=limit).contains(&value) {
        return Err(format!(
            "{name}: {key} is not a finite number from 0 to {limit}"
        ));
    }
    Ok(value)
}

fn whole(name: &str, raw: Option<&Value>, low: u32, high: u32) -> Checked<u32> {
    raw.and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (low..=high).contains(value))
        .ok_or(format!("{name} is not a whole number from {low} to {high}"))
}

fn text(key: &str, raw: Option<&Value>, limit: usize) -> Checked<String> {
    match raw {
        Some(Value::String(text)) if text.chars().count() <= limit => Ok(text.trim().to_string()),
        _ => Err(format!(
            "{key} is not a string of at most {limit} characters"
        )),
    }
}

/// A version string: an ASCII letter or digit, then up to 39 letters, digits, `.`, `+` or `-`.
fn version(name: &str, raw: Option<&Value>) -> Checked<String> {
    let valid = |text: &str| {
        let mut chars = text.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
            && text.len() <= 40
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
    };
    match raw.and_then(Value::as_str) {
        Some(text) if valid(text) => Ok(text.to_string()),
        _ => Err(format!("unexpected version for {name}: {}", show(raw))),
    }
}

/// One score: its status, value, position range, tie margin and known flags.
fn cell(name: &str, raw: Option<&Value>, ranked: u32) -> Checked<Cell> {
    let object = raw.and_then(Value::as_object);
    let status = object
        .and_then(|object| known(&STATUSES, object.get("status")))
        .ok_or_else(|| {
            let shown = object.map_or_else(|| show(raw), |object| show(object.get("status")));
            format!("{name}: unknown status {shown}")
        })?;
    let object = object.ok_or(format!("{name}: unknown status"))?;
    let present = |key: &str| object.get(key).filter(|value| !value.is_null());
    let score = present("score")
        .map(|raw| metric(name, "score", raw, 1000.0))
        .transpose()?;
    let position = match present("position") {
        Some(position) => {
            let pair = position
                .as_array()
                .filter(|pair| pair.len() == 2 && score.is_some());
            let pair = pair.ok_or(format!(
                "{name}: a position is a [first, last] range of a score"
            ))?;
            let label = format!("{name} position");
            let first = whole(&label, pair.first(), 1, ranked)?;
            let last = whole(&label, pair.get(1), 1, ranked)?;
            if first > last {
                return Err(format!(
                    "{name}: position range [{first}, {last}] is reversed"
                ));
            }
            Some((first, last))
        }
        None if status == "ranked" && score.is_some() => {
            return Err(format!("{name}: a ranked score needs a position"));
        }
        None => None,
    };
    let margin = present("margin_percent")
        .map(|raw| metric(name, "margin_percent", raw, 1000.0))
        .transpose()?;
    let flags = match object.get("flags") {
        None => Vec::new(),
        Some(Value::Array(flags)) => flags.iter().collect(),
        Some(other) => return Err(format!("{name}: unknown flag in {other}")),
    };
    let mut known_flags = BTreeSet::new();
    for flag in flags {
        let key = FLAGS
            .iter()
            .map(|(key, _)| *key)
            .find(|key| Some(*key) == flag.as_str())
            .ok_or(format!("{name}: unknown flag in {flag}"))?;
        known_flags.insert(key);
    }
    Ok(Cell {
        status,
        score,
        position,
        margin,
        flags: known_flags,
    })
}

/// The shown row a result belongs to and its checked values, or None for another implementation.
fn row(raw: &Value, ranked: u32, overall_published: bool) -> Checked<Option<Row>> {
    let Some(object) = raw.as_object() else {
        return Ok(None);
    };
    let slugs: Vec<&'static str> = known_rows().map(|(slug, _)| *slug).collect();
    let Some(slug) = known(&slugs, object.get("slug")) else {
        return Ok(None);
    };
    let name = row_name(slug);
    let role = if slug == MINIMAL {
        "diagnostic"
    } else {
        "ranked"
    };
    if object.get("role").and_then(Value::as_str) != Some(role) {
        return Err(format!(
            "{name}: expected the role {role}, not {}",
            show(object.get("role"))
        ));
    }
    let categories = object.get("categories").and_then(Value::as_object);
    let categories = categories
        .filter(|map| {
            map.len() == CATEGORIES.len() && CATEGORIES.iter().all(|key| map.contains_key(*key))
        })
        .ok_or(format!("{name}: the categories are not {CATEGORIES:?}"))?;
    let stability = match object.get("stability") {
        None | Some(Value::Null) => None,
        Some(value) => Some(known(&STABILITY, Some(value)).ok_or(format!(
            "{name}: unknown stability result {}",
            show(Some(value))
        ))?),
    };
    let empty = Map::new();
    let memory = match object.get("memory") {
        Some(Value::Object(map)) => map,
        Some(value) if truthy(value) => return Err(format!("{name}: memory is not an object")),
        _ => &empty,
    };
    let version = version(name, object.get("version"))?;
    let overall = if overall_published {
        Some(cell(
            &format!("{name} overall"),
            object.get("overall"),
            ranked,
        )?)
    } else {
        None
    };
    let categories = CATEGORIES
        .iter()
        .map(|key| cell(&format!("{name} {key}"), categories.get(*key), ranked))
        .collect::<Checked<Vec<_>>>()?;
    let mut figures = Vec::new();
    for key in MEMORY {
        if let Some(raw) = memory.get(key).filter(|value| !value.is_null()) {
            figures.push((key, metric(name, key, raw, 1e9)?));
        }
    }
    Ok(Some(Row {
        slug,
        version,
        overall,
        categories,
        stability,
        memory: figures,
    }))
}

fn same_weights(raw: Option<&Value>) -> bool {
    let Some(map) = raw.and_then(Value::as_object) else {
        return false;
    };
    map.len() == WEIGHTS.len()
        && WEIGHTS.iter().all(|(key, weight)| {
            map.get(*key)
                .filter(|value| value.is_number())
                .and_then(Value::as_f64)
                .is_some_and(|value| value == *weight as f64)
        })
}

fn same_flags(raw: Option<&Value>) -> bool {
    let Some(map) = raw.and_then(Value::as_object) else {
        return false;
    };
    map.len() == FLAGS.len()
        && FLAGS
            .iter()
            .all(|(key, sign)| map.get(*key).and_then(Value::as_str) == Some(*sign))
}

fn same_categories(raw: Option<&Value>) -> bool {
    let Some(list) = raw.and_then(Value::as_array) else {
        return false;
    };
    list.len() == CATEGORIES.len()
        && list.iter().zip(CATEGORIES).all(|(item, key)| {
            let named = item
                .as_object()
                .map_or(Some(item), |object| object.get("key"));
            named.and_then(Value::as_str) == Some(key)
        })
}

/// Every value the pages show, checked; an error for anything the pages do not know.
pub fn results(data: &Value, today: Date, max_age_days: i64) -> Checked<Results> {
    let data = data
        .as_object()
        .filter(|object| object.get("schema").and_then(Value::as_str) == Some(super::SCHEMA))
        .ok_or(format!("schema is not {SCHEMA}"))?;
    let measured = data.get("measured_on").and_then(Value::as_str);
    let measured_on = measured.and_then(Date::parse).ok_or(format!(
        "measured_on is not a YYYY-MM-DD date: {}",
        show(data.get("measured_on"))
    ))?;
    if measured_on > today {
        return Err(format!(
            "measured_on {} is after today ({})",
            measured_on.iso(),
            today.iso()
        ));
    }
    if today.days() - measured_on.days() > max_age_days {
        return Err(format!(
            "the results were measured on {}, more than {max_age_days} days ago",
            measured_on.iso()
        ));
    }
    let run = data.get("run");
    let run_url = run
        .and_then(Value::as_object)
        .and_then(|run| run.get("url"))
        .and_then(Value::as_str)
        .filter(|url| {
            url.strip_prefix(RUN_PREFIX).is_some_and(|id| {
                (1..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
            })
        })
        .ok_or(format!("unexpected run url: {}", show(run)))?;
    let reference = data.get("reference").and_then(Value::as_object);
    let reference = reference
        .filter(|reference| {
            reference.get("slug").and_then(Value::as_str) == Some("axum")
                && reference.get("framework").and_then(Value::as_str) == Some("Axum")
        })
        .ok_or("the reference is not Axum")?;
    let ranked = whole("ranked", data.get("ranked"), 1, 1000)?;
    let overall_published = data
        .get("overall_published")
        .and_then(Value::as_bool)
        .ok_or("overall_published is not true or false")?;
    if overall_published && !same_weights(data.get("weights")) {
        return Err(format!("the overall weights are not {WEIGHTS:?}"));
    }
    if !same_categories(data.get("categories")) {
        return Err(format!("the categories are not {CATEGORIES:?}"));
    }
    if !same_flags(data.get("flags")) {
        return Err(format!("the flags are not {FLAGS:?}"));
    }
    let raw_rows = data
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("rows is not a list")?;
    let mut rows: Vec<Row> = Vec::new();
    for raw in raw_rows {
        if let Some(row) = row(raw, ranked, overall_published)? {
            if rows.iter().any(|seen| seen.slug == row.slug) {
                return Err(format!("{} appears twice", row_name(row.slug)));
            }
            rows.push(row);
        }
    }
    let source = data
        .get("rullst_source")
        .and_then(Value::as_object)
        .and_then(|source| known(&["crates.io", "git"], source.get("kind")));
    let reading = data
        .get("reading")
        .and_then(Value::as_object)
        .ok_or("reading is not an object")?;
    let reading_en = text("reading.en", reading.get("en"), READING_LIMIT)?;
    let reading_pt = text("reading.pt_br", reading.get("pt_br"), READING_LIMIT)?;
    if reading_en.is_empty() || reading_pt.is_empty() {
        return Err("the reading is empty".to_string());
    }
    let rullst_version = version("Rullst", data.get("rullst_version"))?;
    let axum_version = version("Axum", reference.get("version"))?;
    let logins = match data.get("logins") {
        None | Some(Value::Null) => None,
        raw => Some(whole("logins", raw, 1, 100_000)?),
    };
    let note = |key: &str| -> Checked<Option<String>> {
        match data.get(key) {
            Some(value) if truthy(value) => text(key, Some(value), NOTE_LIMIT).map(Some),
            _ => Ok(None),
        }
    };
    Ok(Results {
        measured_on,
        rullst_version,
        source,
        run_id: run_url.trim_start_matches(RUN_PREFIX).to_string(),
        run_url: run_url.to_string(),
        axum_version,
        ranked,
        overall_published,
        logins,
        rows,
        note_en: note("note")?,
        note_pt: note("note_pt_br")?,
        reading_en,
        reading_pt,
    })
}
