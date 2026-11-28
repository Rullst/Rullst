//! The comparison pages' results block: meta line, note, scores table, legend, memory table
//! and the reading for Rullst.

use rullst::html;
use rullst::html::Markup;

use super::format::{number, render_note, score};
use super::text::{self, words};
use super::{Cell, FLAGS, MINIMAL, ROWS, Results, Row, known_rows};
use crate::pages::Lang;

/// Measurement runs of Rullst/Benchmarks whose results an automatic run would not
/// publish, as their results page says: the run id, the number of problems and their
/// cause in English and Portuguese. A run that is not listed shows no gate note.
const GATE_FAILURES: [(&str, u32, &str, &str); 1] = [(
    "44765309608",
    18,
    "its `/world-update` runs recorded no count of the rows they updated",
    "as execuções de `/world-update` não registraram quantas linhas atualizaram",
)];

/// The note that the shown run failed the benchmark's own publication gate, when it is
/// one of [`GATE_FAILURES`].
pub fn gate_note(results: &Results, lang: Lang) -> Option<Markup> {
    let (_, problems, cause_en, cause_pt) = GATE_FAILURES
        .iter()
        .find(|(run, ..)| *run == results.run_id)?;
    Some(if lang.is_pt() {
        html! { <p class="benchmark-note benchmark-gate"><strong>"Porta de publicação: "</strong>"uma execução automática do Rullst/Benchmarks não publicaria estes resultados ("{problems.to_string()}" problemas): "{render_note(cause_pt)}". A "<a href="/Benchmarks/">"página de resultados"</a>" (em inglês) lista os problemas e diz quais números eles afetam."</p> }
    } else {
        html! { <p class="benchmark-note benchmark-gate"><strong>"Publication gate: "</strong>"an automatic run of Rullst/Benchmarks would not publish these results ("{problems.to_string()}" problems): "{render_note(cause_en)}". The "<a href="/Benchmarks/">"results page"</a>" lists the problems and says which numbers they affect."</p> }
    })
}

/// The measurement day as a `<time>` element in the page's language.
pub fn measured_date(results: &Results, lang: Lang) -> Markup {
    let day = results.measured_on;
    html! { <time datetime={day.iso()}>{day.long(lang.is_pt())}</time> }
}

/// Date, Rullst version and source, Axum's version and the measurement run.
pub fn measured_line(results: &Results, lang: Lang) -> Markup {
    let run = html! { <a href={results.run_url.as_str()}>{words(lang).run}" "{results.run_id.as_str()}</a> };
    let date = measured_date(results, lang);
    let version = results.rullst_version.as_str();
    let axum = results.axum_version.as_str();
    if lang.is_pt() {
        let source = match results.source {
            Some("crates.io") => " do crates.io",
            Some("git") => " de uma revisão git",
            _ => "",
        };
        html! { "Medido em "{date}" com o Rullst "{version}{source}", contra o Axum "{axum}" nas mesmas máquinas do GitHub ("{run}")." }
    } else {
        let source = match results.source {
            Some("crates.io") => " from crates.io",
            Some("git") => " from a git revision",
            _ => "",
        };
        html! { "Measured on "{date}" with Rullst "{version}{source}", against Axum "{axum}" on the same GitHub-hosted runners ("{run}")." }
    }
}

fn full_results(ranked: u32, lang: Lang) -> Markup {
    if lang.is_pt() {
        html! { "Os "<a href="/Benchmarks/">"resultados completos"</a>" (em inglês) listam as "{ranked}" implementações classificadas, cada endpoint e o método. Eles se atualizam automaticamente a cada nova versão do Rullst; esta página é reconstruída diariamente." }
    } else {
        html! { "The "<a href="/Benchmarks/">"full results"</a>" list all "{ranked}" ranked implementations, every endpoint and the method. They update automatically after each new Rullst release; this page is rebuilt daily." }
    }
}

/// Position, score with its flags, and tie margin; the status when there is no score.
fn cell(cell: &Cell, lang: Lang) -> Markup {
    if cell.score.is_none() {
        return html! { <td class="benchmark-status">{text::status(cell.status, lang)}</td> };
    }
    let position = match cell.position {
        None => "–".to_string(),
        Some((first, last)) if first == last => first.to_string(),
        Some((first, last)) => format!("{first}–{last}"),
    };
    let margin = cell.margin.map(
        |margin| html! { <span class="benchmark-margin">"±"{number(margin, 1, lang)}"%"</span> },
    );
    html! { <td><span class="benchmark-position">{position}</span><span class="benchmark-score">{score(cell, lang, true)}</span>{margin}</td> }
}

fn row_header(slug: &str, row: Option<&Row>, lang: Lang) -> Markup {
    let variant = match slug {
        "rullst" if lang.is_pt() => Some(html! { "como o "<code>"cargo rullst new"</code>" gera" }),
        "rullst" => Some(html! { "as "<code>"cargo rullst new"</code>" generates it" }),
        MINIMAL if lang.is_pt() => Some(html! { "diagnóstico, sem posição" }),
        MINIMAL => Some(html! { "diagnostic, no position" }),
        _ => None,
    };
    let version = row.map(|row| html! { {row.version.as_str()} });
    let details: Vec<Markup> = version.into_iter().chain(variant).collect();
    let detail = (!details.is_empty()).then(|| {
        let joined: Markup = details
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part
                } else {
                    html! { " · "{part} }
                }
            })
            .collect();
        html! { <span class="benchmark-variant">{joined}</span> }
    });
    html! { <th scope="row">{words(lang).row_name(slug)}{detail}</th> }
}

/// Measured rows by overall position (the fixed order without a published overall), then
/// the compared frameworks missing from the run, then the diagnostic row. A framework of
/// another language is shown only when the run has it.
pub fn shown_rows(results: &Results) -> Vec<&'static str> {
    let mut order: Vec<&Row> = known_rows()
        .filter_map(|(slug, _)| results.row(slug))
        .collect();
    if results.overall_published {
        order.sort_by(|a, b| {
            let key = |row: &Row| {
                let overall = row.overall.as_ref();
                let first = overall
                    .and_then(|cell| cell.position)
                    .map_or(1_000_000, |(first, _)| first);
                let score = overall.and_then(|cell| cell.score).unwrap_or(0.0);
                (row.slug == MINIMAL, first, -score)
            };
            let (a, b) = (key(a), key(b));
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
    }
    let missing = ROWS
        .iter()
        .map(|(slug, _)| *slug)
        .filter(|slug| *slug != MINIMAL && results.row(slug).is_none());
    order
        .iter()
        .filter(|row| row.slug != MINIMAL)
        .map(|row| row.slug)
        .chain(missing)
        .chain(
            order
                .iter()
                .filter(|row| row.slug == MINIMAL)
                .map(|row| row.slug),
        )
        .collect()
}

fn scores_table(results: &Results, order: &[&str], lang: Lang) -> Markup {
    let words = words(lang);
    let headers: Vec<&str> = words
        .headers
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != 1 || results.overall_published)
        .map(|(_, name)| *name)
        .collect();
    let head: Markup = headers
        .iter()
        .map(|name| html! { <th scope="col">{*name}</th> })
        .collect();
    let span = headers.len() - 1;
    let lines: Markup = order
        .iter()
        .map(|slug| match results.row(slug) {
            None => html! { <tr>{row_header(slug, None, lang)}<td colspan={span}>{words.missing}</td></tr> },
            Some(row) => {
                let cells: Markup = row.cells().map(|item| cell(item, lang)).collect();
                let stability = row.stability.map_or("–", |result| text::stability(result, lang));
                let class = (row.slug == MINIMAL).then_some("benchmark-diagnostic");
                html! { <tr class?={class}>{row_header(slug, Some(row), lang)}{cells}<td>{stability}</td></tr> }
            }
        })
        .collect();
    let caption = if lang.is_pt() {
        format!(
            "Pontuação em relação ao Axum (1,00×; mais é melhor em todas as colunas), posição entre as {} implementações classificadas e margem de empate (±).",
            results.ranked
        )
    } else {
        format!(
            "Score relative to Axum (1.00×; higher is better in every column), position among the {} ranked implementations and tie margin (±).",
            results.ranked
        )
    };
    // The caption stays outside the horizontally scrolling region, readable at any width.
    html! {
        <p class="benchmark-caption" id="benchmark-caption">{caption}</p>
        <div class="benchmark-table" role="region" aria-labelledby="benchmark-caption" tabindex="0">
            <table aria-labelledby="benchmark-caption"><thead><tr>{head}</tr></thead><tbody>{lines}</tbody></table>
        </div>
    }
}

fn memory_table(results: &Results, order: &[&str], lang: Lang) -> Markup {
    let logins = results
        .logins
        .map_or("–".to_string(), |logins| logins.to_string());
    let pair = |row: &Row, own: &str, axum: &str| -> Markup {
        let Some(value) = row.memory(own) else {
            return html! { "–" };
        };
        let reference = row.memory(axum).map(
            |axum| html! { " "<span class="benchmark-axum">"("{number(axum, 0, lang)}")"</span> },
        );
        html! { {number(value, 0, lang)}{reference} }
    };
    let lines: Markup = order
        .iter()
        .filter_map(|slug| results.row(slug))
        .map(|row| {
            html! { <tr>{row_header(row.slug, Some(row), lang)}<td>{pair(row, "peak_ram_mib", "axum_peak_ram_mib")}</td><td>{pair(row, "after_logins_peak_ram_mib", "axum_after_logins_peak_ram_mib")}</td></tr> }
        })
        .collect();
    let (headers, caption) = if lang.is_pt() {
        (
            [
                "Framework".to_string(),
                "Pico sob carga".to_string(),
                format!("Pico depois dos {logins} logins"),
            ],
            format!(
                "RAM do servidor em MiB, com a do Axum nas mesmas máquinas entre parênteses. A pontuação de Memória compara o pico sob carga; nenhuma pontuação inclui o pico depois dos {logins} logins dos endpoints de aplicação."
            ),
        )
    } else {
        (
            [
                "Framework".to_string(),
                "High-water mark under load".to_string(),
                format!("Peak after the {logins} logins"),
            ],
            format!(
                "RAM of the server in MiB, with Axum's on the same runners in parentheses. The Memory score compares the high-water mark under load; no score includes the peak after the {logins} logins of the application endpoints."
            ),
        )
    };
    let head: Markup = headers
        .iter()
        .map(|name| html! { <th scope="col">{name.as_str()}</th> })
        .collect();
    html! {
        <h3>{words(lang).memory}</h3>
        <p class="benchmark-caption" id="benchmark-memory-caption">{caption}</p>
        <div class="benchmark-table benchmark-memory" role="region" aria-labelledby="benchmark-memory-caption" tabindex="0">
            <table aria-labelledby="benchmark-memory-caption"><thead><tr>{head}</tr></thead><tbody>{lines}</tbody></table>
        </div>
    }
}

/// The comparison page's results block, every value escaped by `html!`.
pub fn render(results: &Results, lang: Lang) -> Markup {
    let words = words(lang);
    // The English note stands in, marked as English, when there is no Portuguese one.
    let (note, english) = match (lang.is_pt(), &results.note_pt, &results.note_en) {
        (true, Some(note), _) => (Some(note), false),
        (true, None, Some(note)) => (Some(note), true),
        (false, _, note) => (note.as_ref(), false),
        (true, None, None) => (None, false),
    };
    let note = note
        .filter(|note| !note.is_empty())
        .map(|note| html! { <p class="benchmark-note" lang?={english.then_some("en")}>{render_note(note)}</p> });
    let order = shown_rows(results);
    let shown: Vec<&str> = FLAGS
        .iter()
        .map(|(flag, _)| *flag)
        .filter(|flag| {
            results
                .rows
                .iter()
                .any(|row| row.cells().any(|cell| cell.has(flag)))
        })
        .collect();
    let unpublished = (!results.overall_published).then(|| html! { <li>{words.unpublished}</li> });
    let flags: Markup = shown
        .iter()
        .map(|flag| html! { <li>{words.flag(flag)}</li> })
        .collect();
    let reading = if lang.is_pt() {
        &results.reading_pt
    } else {
        &results.reading_en
    };
    html! {
        <p class="benchmark-meta">{measured_line(results, lang)}" "{full_results(results.ranked, lang)}</p>
        {gate_note(results, lang)}
        {note}
        {scores_table(results, &order, lang)}
        <ul class="benchmark-legend"><li>{words.legend}</li>{unpublished}{flags}</ul>
        {memory_table(results, &order, lang)}
        <h3>{words.reading}</h3>
        <p class="benchmark-reading">{render_note(reading)}</p>
    }
}
