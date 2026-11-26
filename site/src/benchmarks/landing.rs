//! The landing's two benchmark blocks: the first screen's memory proof point and the
//! Performance summary of the application `cargo rullst new` generates.

use rullst::html;
use rullst::html::Markup;

use super::compare::{gate_note, measured_date, measured_line};
use super::format::{mib, score};
use super::text::{self, words};
use super::{CATEGORIES, Cell, FLAGS, MINIMAL, Results, Row, SPEED, row_name};
use crate::pages::Lang;

/// The blocks the data can fill: the summary, and the proof point when Rullst's RAM was
/// measured. None, after a warning, when the data has no Rullst row to summarize.
pub struct Blocks {
    pub proof: Option<Markup>,
    pub summary: Markup,
}

pub fn render(results: &Results, lang: Lang) -> Option<Blocks> {
    let row = results.row("rullst")?;
    let measured =
        row.memory("peak_ram_mib").is_some() && row.memory("axum_peak_ram_mib").is_some();
    Some(Blocks {
        proof: measured.then(|| proof(results, row, lang)),
        summary: summary(results, row, lang),
    })
}

/// Rullst's RAM under load and after the logins, both with Axum's.
struct Memory {
    peak: String,
    axum_peak: String,
    /// The peak after the logins: (Rullst's, Axum's, the number of logins).
    after: Option<(String, String, u32)>,
}

fn memory(results: &Results, row: &Row, lang: Lang) -> Option<Memory> {
    let after = match (
        results.logins,
        row.memory("after_logins_peak_ram_mib"),
        row.memory("axum_after_logins_peak_ram_mib"),
    ) {
        (Some(logins), Some(after), Some(axum)) => {
            Some((mib(after, lang), mib(axum, lang), logins))
        }
        _ => None,
    };
    Some(Memory {
        peak: mib(row.memory("peak_ram_mib")?, lang),
        axum_peak: mib(row.memory("axum_peak_ram_mib")?, lang),
        after,
    })
}

/// The first screen's proof point: Rullst's RAM against Axum's and its lowest speed score.
fn proof(results: &Results, row: &Row, lang: Lang) -> Markup {
    let Some(Memory {
        peak,
        axum_peak,
        after,
    }) = memory(results, row, lang)
    else {
        return Markup::default();
    };
    let pt = lang.is_pt();
    let ram = match (after, pt) {
        (Some((after, axum_after, logins)), false) => {
            html! { <strong>{peak}" MiB"</strong>" of RAM at its high-water mark under load and "{after}" MiB after "{logins}" logins; Axum: "{axum_peak}" and "{axum_after}" MiB on the same runners (the RAM after the logins is in no score)." }
        }
        (Some((after, axum_after, logins)), true) => {
            html! { <strong>{peak}" MiB"</strong>" de RAM no pico sob carga e "{after}" MiB depois de "{logins}" logins; Axum: "{axum_peak}" e "{axum_after}" MiB nas mesmas máquinas (a RAM depois dos logins não entra em nenhuma pontuação)." }
        }
        (None, false) => {
            html! { <strong>{peak}" MiB"</strong>" of RAM at its high-water mark under load; Axum: "{axum_peak}" MiB on the same runners." }
        }
        (None, true) => {
            html! { <strong>{peak}" MiB"</strong>" de RAM no pico sob carga; Axum: "{axum_peak}" MiB nas mesmas máquinas." }
        }
    };
    // The Memory score, which can call a smaller figure a tie with Axum.
    let memory_score = row
        .category("memory")
        .filter(|cell| cell.score.is_some())
        .map(|cell| {
            let value = score(cell, lang, false);
            match (pt, cell.has("within_reference_margin")) {
                (false, true) => html! { " Its Memory score is "{value}": a tie with Axum." },
                (false, false) => html! { " Its Memory score is "{value}" Axum's." },
                (true, true) => {
                    html! { " A pontuação de Memória é "{value}": um empate com o Axum." }
                }
                (true, false) => html! { " A pontuação de Memória é "{value}" a do Axum." },
            }
        });
    // The lowest throughput score (the first one on a tie).
    let mut lowest: Option<(&str, &Cell)> = None;
    for key in SPEED {
        if let Some(cell) = row.category(key).filter(|cell| cell.score.is_some())
            && lowest.is_none_or(|(_, low)| cell.score < low.score)
        {
            lowest = Some((key, cell));
        }
    }
    let speed = lowest.map(|(key, cell)| {
        let category = words(lang).category(key);
        let flagged = cell.has("put_not_written");
        let value = score(cell, lang, false) + if flagged { " ‡" } else { "" };
        match (pt, flagged) {
            (false, false) => html! { " Its lowest speed score is "{category}": "{value}" Axum's." },
            (false, true) => html! { " Its lowest speed score is "{category}": "{value}" Axum's. (‡: see Performance)" },
            (true, false) => html! { " A menor pontuação de velocidade é a de "{category}": "{value}" a do Axum." },
            (true, true) => html! { " A menor pontuação de velocidade é a de "{category}": "{value}" a do Axum. (‡: veja Desempenho)" },
        }
    });
    let date = measured_date(results, lang);
    let meta = if pt {
        html! { "Medido pelo Rullst/Benchmarks em "{date}"." }
    } else {
        html! { "Measured by Rullst/Benchmarks on "{date}"." }
    };
    html! { <p class="proof-figure">{ram}{memory_score}{speed}</p><p class="proof-meta">{meta}</p> }
}

fn positions(cell: &Cell, ranked: u32, lang: Lang) -> String {
    let (first, last) = cell.position.unwrap_or_default();
    match (first == last, lang.is_pt()) {
        (false, false) => format!("positions {first}–{last} of {ranked}"),
        (true, false) => format!("position {first} of {ranked}"),
        (false, true) => format!("posições {first}–{last} de {ranked}"),
        (true, true) => format!("posição {first} de {ranked}"),
    }
}

/// Who shares Rullst's overall position range, as far as the shown rows tell.
fn group(results: &Results, overall: &Cell, lang: Lang) -> String {
    let pt = lang.is_pt();
    let (first, last) = overall.position.unwrap_or_default();
    if first == last {
        return if pt {
            "Nenhuma outra implementação divide a posição dele."
        } else {
            "No other implementation shares its position."
        }
        .to_string();
    }
    let mut members: Vec<(&str, &Cell)> = results
        .rows
        .iter()
        .filter(|row| row.slug != "rullst" && row.slug != MINIMAL)
        .filter_map(|row| row.overall.as_ref().map(|cell| (row.slug, cell)))
        .filter(|(_, cell)| cell.position == Some((first, last)) && cell.score.is_some())
        .collect();
    // Highest score first; a stable sort keeps the data's order on a tie.
    members.sort_by(|a, b| {
        b.1.score
            .partial_cmp(&a.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let names: Vec<String> = members
        .iter()
        .map(|(slug, cell)| format!("{} ({})", row_name(slug), score(cell, lang, false)))
        .collect();
    let joined = match names.split_last() {
        Some((last_name, rest)) if !rest.is_empty() => {
            format!(
                "{}{}{last_name}",
                rest.join(", "),
                if pt { " e " } else { " and " }
            )
        }
        _ => names.concat(),
    };
    let others = last - first;
    match (
        names.is_empty(),
        names.len() < others as usize,
        others == 1,
        pt,
    ) {
        (true, _, true, false) => format!(
            "It shares positions {first}–{last} with one other implementation of the full results."
        ),
        (true, _, true, true) => format!(
            "Divide as posições {first}–{last} com outra implementação dos resultados completos."
        ),
        (true, _, false, false) => format!(
            "It shares positions {first}–{last} with {others} other implementations of the full results."
        ),
        (true, _, false, true) => format!(
            "Divide as posições {first}–{last} com outras {others} implementações dos resultados completos."
        ),
        (false, true, _, false) => format!(
            "It shares positions {first}–{last} with {others} other implementations, among them {joined}."
        ),
        (false, true, _, true) => format!(
            "Divide as posições {first}–{last} com outras {others} implementações, entre elas {joined}."
        ),
        (false, false, _, false) => format!("It shares positions {first}–{last} with {joined}."),
        (false, false, _, true) => format!("Divide as posições {first}–{last} com {joined}."),
    }
}

fn item(name: &str, cell: &Cell, ranked: u32, lang: Lang) -> Markup {
    if cell.score.is_none() {
        return html! { <li><span class="landing-bench-category">{name}</span><span class="landing-bench-value">{text::status(cell.status, lang)}</span></li> };
    }
    let position = cell.position.map(
        |_| html! { <span class="landing-bench-position">{positions(cell, ranked, lang)}</span> },
    );
    html! { <li><span class="landing-bench-category">{name}</span><span class="landing-bench-value">{score(cell, lang, true)}</span>{position}</li> }
}

/// The landing's Performance summary of the generated Rullst application.
fn summary(results: &Results, row: &Row, lang: Lang) -> Markup {
    let pt = lang.is_pt();
    let words = words(lang);
    let ranked = results.ranked;
    let mut shown: Vec<&Cell> = Vec::new();
    let overall = match row.overall.as_ref().filter(|_| results.overall_published) {
        Some(overall) => {
            shown.push(overall);
            let label = if pt {
                "Pontuação geral"
            } else {
                "Overall score"
            };
            let figure = match (overall.score, overall.position) {
                (None, _) => {
                    html! { <strong class="landing-bench-score">{text::status(overall.status, lang)}</strong> }
                }
                (Some(_), None) => {
                    html! { <strong class="landing-bench-score">{score(overall, lang, true)}</strong> }
                }
                (Some(_), Some(_)) => html! {
                    <strong class="landing-bench-score">{score(overall, lang, true)}</strong>
                    <span class="landing-bench-position">{positions(overall, ranked, lang)}</span>" "
                    <span class="landing-bench-group">{group(results, overall, lang)}</span>
                },
            };
            html! { <p class="landing-bench-overall"><span class="landing-bench-label">{label}</span>{figure}</p> }
        }
        None => html! { <p class="landing-bench-overall">{words.unpublished}</p> },
    };
    let (mut level, mut below, mut unscored) = (Vec::new(), Vec::new(), Vec::new());
    for (key, cell) in CATEGORIES.iter().zip(row.categories.iter()) {
        shown.push(cell);
        let line = item(words.category(key), cell, ranked, lang);
        match cell.score {
            None => unscored.push(line),
            Some(value) if cell.has("within_reference_margin") || value >= 1.0 => level.push(line),
            Some(_) => below.push(line),
        }
    }
    let none = if pt {
        "Nenhuma nesta rodada."
    } else {
        "None in this run."
    };
    let column = |kind: &str, title: &str, items: Vec<Markup>| -> Markup {
        let items: Markup = if items.is_empty() {
            html! { <li class="landing-bench-none">{none}</li> }
        } else {
            items.into_iter().collect()
        };
        html! { <div class={format!("landing-bench-column landing-bench-{kind}")}><h3>{title}</h3><ul>{items}</ul></div> }
    };
    let level = column(
        "level",
        if pt {
            "No nível do Axum ou acima"
        } else {
            "Level with or above Axum"
        },
        level,
    );
    let below = column(
        "below",
        if pt { "Abaixo do Axum" } else { "Below Axum" },
        below,
    );
    let unscored = (!unscored.is_empty()).then(|| {
        column(
            "unscored",
            if pt {
                "Sem pontuação"
            } else {
                "Without a score"
            },
            unscored,
        )
    });
    let ram = memory(results, row, lang).map(|Memory { peak, axum_peak, after }| match (after, pt) {
        (Some((after, axum_after, logins)), false) => format!("RAM: {peak} MiB at the high-water mark under load (Axum: {axum_peak} MiB); {after} MiB after the {logins} logins of the application endpoints (Axum: {axum_after} MiB), which no score includes."),
        (Some((after, axum_after, logins)), true) => format!("RAM: {peak} MiB no pico sob carga (Axum: {axum_peak} MiB); {after} MiB depois dos {logins} logins dos endpoints de aplicação (Axum: {axum_after} MiB), que nenhuma pontuação inclui."),
        (None, false) => format!("RAM: {peak} MiB at the high-water mark under load (Axum: {axum_peak} MiB)."),
        (None, true) => format!("RAM: {peak} MiB no pico sob carga (Axum: {axum_peak} MiB)."),
    });
    let stability = row.stability.map(|result| {
        let result = text::stability(result, lang);
        if pt {
            format!("Estabilidade sob carga mais pesada: {result}.")
        } else {
            format!("Stability under heavier load: {result}.")
        }
    });
    let facts: Vec<String> = ram.into_iter().chain(stability).collect();
    let facts =
        (!facts.is_empty()).then(|| html! { <p class="landing-bench-facts">{facts.join(" ")}</p> });
    let minimal_cells: Vec<(&str, &Cell)> = results
        .row(MINIMAL)
        .map(|minimal| {
            SPEED
                .iter()
                .filter_map(|key| {
                    minimal
                        .category(key)
                        .filter(|cell| cell.score.is_some())
                        .map(|cell| (*key, cell))
                })
                .collect()
        })
        .unwrap_or_default();
    shown.extend(minimal_cells.iter().map(|(_, cell)| *cell));
    let minimal = (!minimal_cells.is_empty()).then(|| {
        let listed = minimal_cells
            .iter()
            .map(|(key, cell)| format!("{} {}", words.category(key), score(cell, lang, true)))
            .collect::<Vec<_>>()
            .join(", ");
        if pt {
            html! { <p class="landing-bench-minimal">"A linha de diagnóstico “Rullst (mínimo)”, o mesmo roteador, ORM e templates "<code>"html!"</code>" servidos sem esse middleware e sem posição, pontua "{listed}"; a diferença entre as duas linhas é o custo dessas camadas."</p> }
        } else {
            html! { <p class="landing-bench-minimal">"The diagnostic “Rullst (minimal)” row, the same router, ORM and "<code>"html!"</code>" templates served without that middleware and without a position, scores "{listed}"; the gap between the two rows is what those layers cost."</p> }
        }
    });
    let legend = if pt {
        "1,00× é o Axum, e mais é melhor em todas as pontuações. Um intervalo como 4–5 é um empate: essas implementações dividem as posições, sem ordem entre elas."
    } else {
        "1.00× is Axum and higher is better in every score. A range such as 4–5 is a tie: those implementations share the positions in no particular order."
    };
    let flags: Markup = FLAGS
        .iter()
        .filter(|(flag, _)| shown.iter().any(|cell| cell.has(flag)))
        .map(|(flag, _)| html! { <li>{words.flag(flag)}</li> })
        .collect();
    html! {
        <p class="landing-bench-meta">{measured_line(results, lang)}</p>
        {gate_note(results, lang)}
        {overall}
        <div class="landing-bench-split">{level}{below}{unscored}</div>
        {facts}
        {minimal}
        {meaning(results, row, lang)}
        <ul class="benchmark-legend landing-bench-legend"><li>{legend}</li>{flags}</ul>
    }
}

/// "In plain words": what the numbers mean for a real application, every sentence chosen
/// from the data (nothing about a result is written by hand).
fn meaning(results: &Results, row: &Row, lang: Lang) -> Markup {
    let pt = lang.is_pt();
    let words = words(lang);
    let logins = results.logins;
    let logged_in = match (
        logins,
        row.memory("after_logins_peak_ram_mib"),
        row.memory("axum_after_logins_peak_ram_mib"),
    ) {
        (Some(logins), Some(own), Some(axum)) => {
            let (own_text, axum_text) = (mib(own, lang), mib(axum, lang));
            let line = match (own < axum, pt) {
                (true, false) => format!(
                    "With signed-in users: after {logins} logins the generated application held {own_text} MiB, less than Axum's {axum_text} MiB. That is closer to a running service than an idle process; no score includes it."
                ),
                (false, false) => format!(
                    "With signed-in users: after {logins} logins the generated application held {own_text} MiB, not less than Axum's {axum_text} MiB. That is closer to a running service than an idle process; no score includes it."
                ),
                (true, true) => format!(
                    "Com usuários autenticados: depois de {logins} logins, a aplicação gerada ocupava {own_text} MiB, menos que os {axum_text} MiB do Axum. Isso está mais perto de um serviço em funcionamento do que de um processo parado; nenhuma pontuação o inclui."
                ),
                (false, true) => format!(
                    "Com usuários autenticados: depois de {logins} logins, a aplicação gerada ocupava {own_text} MiB, não menos que os {axum_text} MiB do Axum. Isso está mais perto de um serviço em funcionamento do que de um processo parado; nenhuma pontuação o inclui."
                ),
            };
            Some(line)
        }
        _ => None,
    };
    let startup = row
        .category("startup")
        .filter(|cell| cell.score.is_some())
        .map(|cell| {
            let value = score(cell, lang, false);
            let level = cell.has("within_reference_margin") || cell.score >= Some(1.0);
            match (level, pt) {
                (true, false) => format!("Startup: {value} Axum's, level with or ahead of it: the time from launching the program to answering its first request."),
                (false, false) => format!("Startup: {value} Axum's, behind it: the time from launching the program to answering its first request."),
                (true, true) => format!("Inicialização: {value} a do Axum, no mesmo nível ou à frente: o tempo entre iniciar o programa e responder a primeira requisição."),
                (false, true) => format!("Inicialização: {value} a do Axum, atrás dele: o tempo entre iniciar o programa e responder a primeira requisição."),
            }
        });
    let hello = |slug: &str| {
        results
            .row(slug)
            .and_then(|row| row.category("hello"))
            .filter(|cell| cell.score.is_some())
            .map(|cell| score(cell, lang, false))
    };
    let cost = hello("rullst").map(|own| match (hello(MINIMAL), pt) {
        (Some(minimal), false) => format!("The price of the security layers: every request passes through the WAF, CSRF check, secure headers and access log that are on by default. On the smallest request (Hello world), where they weigh most, the generated application scores {own} Axum's and the same application without them {minimal}. Fewer requests per second on a busy small machine is what you pay for those protections."),
        (None, false) => format!("The price of the security layers: every request passes through the WAF, CSRF check, secure headers and access log that are on by default. On the smallest request (Hello world), where they weigh most, the generated application scores {own} Axum's. Fewer requests per second on a busy small machine is what you pay for those protections."),
        (Some(minimal), true) => format!("O preço das camadas de segurança: toda requisição passa pelo WAF, pela verificação CSRF, pelos cabeçalhos seguros e pelo log de acesso que vêm ligados por padrão. Na menor requisição (Hello world), onde elas pesam mais, a aplicação gerada pontua {own} a do Axum e a mesma aplicação sem elas {minimal}. Menos requisições por segundo numa máquina pequena e ocupada é o que se paga por essas proteções."),
        (None, true) => format!("O preço das camadas de segurança: toda requisição passa pelo WAF, pela verificação CSRF, pelos cabeçalhos seguros e pelo log de acesso que vêm ligados por padrão. Na menor requisição (Hello world), onde elas pesam mais, a aplicação gerada pontua {own} a do Axum. Menos requisições por segundo numa máquina pequena e ocupada é o que se paga por essas proteções."),
    });
    let behind: Vec<String> = SPEED
        .iter()
        .filter_map(|key| {
            row.category(key)
                .filter(|cell| {
                    cell.score.is_some_and(|value| value < 1.0)
                        && !cell.has("within_reference_margin")
                })
                .map(|cell| format!("{} {}", words.category(key), score(cell, lang, false)))
        })
        .collect();
    let behind = (!behind.is_empty()).then(|| {
        let listed = behind.join(", ");
        if pt {
            format!("Onde o Rullst está atrás do Axum: {listed} (a do Axum). Se a sua aplicação espera muito tráfego numa máquina pequena, isso importa; com pouco tráfego, o banco de dados costuma ser o limite antes do framework.")
        } else {
            format!("Where Rullst is behind Axum: {listed} (Axum's). If your application expects heavy traffic on a small machine this matters; at modest traffic the database usually limits speed before the framework does.")
        }
    });
    let plans = if pt {
        html! { "O que a 13.1 mira: o custo da pilha de middleware gerada, os laços de "<code>"html!"</code>" e gravações sem uma transação por linha, cada correção com números de antes e depois deste benchmark e sem enfraquecer nenhuma camada de segurança (" <a href="https://github.com/Rullst/Rullst/issues/832">"issue #832"</a>")." }
    } else {
        html! { "What 13.1 targets: the cost of the generated middleware stack, "<code>"html!"</code>" loops and writes without a transaction per row, each fix with before-and-after numbers from this benchmark and no weakened security layer ("<a href="https://github.com/Rullst/Rullst/issues/832">"issue #832"</a>")." }
    };
    let lines: Vec<String> = logged_in
        .into_iter()
        .chain(startup)
        .chain(cost)
        .chain(behind)
        .collect();
    let items: Markup = lines
        .iter()
        .map(|line| html! { <li>{line.as_str()}</li> })
        .collect();
    let title = if pt {
        "Em palavras simples"
    } else {
        "In plain words"
    };
    html! { <div class="landing-meaning"><h3>{title}</h3><ul>{items}<li>{plans}</li></ul></div> }
}
