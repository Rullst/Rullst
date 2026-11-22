//! The landing page in both languages, split into one module per half of each page.

mod art;
mod en_bottom;
mod en_top;
mod pt_bottom;
mod pt_top;

use rullst::html;
use rullst::html::Markup;

use super::{Context, Lang};
use crate::benchmarks::landing::Blocks;

/// The landing in `lang`, with the benchmark blocks the data filled, or their pending text.
pub fn page(cx: &Context, lang: Lang, blocks: Option<Blocks>) -> Markup {
    let (proof, summary) =
        blocks.map_or((None, None), |blocks| (blocks.proof, Some(blocks.summary)));
    let pt = lang.is_pt();
    let proof = proof.unwrap_or_else(|| {
        if pt {
            html! { <p class="benchmark-pending proof-pending">"Resultados pendentes: este cartão mostra a memória medida só a partir de resultados atuais do Rullst/Benchmarks, nunca números antigos."</p> }
        } else {
            html! { <p class="benchmark-pending proof-pending">"Results pending: this card shows measured memory only from current Rullst/Benchmarks results, never older numbers."</p> }
        }
    });
    let summary = summary.unwrap_or_else(|| {
        if pt {
            html! { <p class="benchmark-pending">"Não há resultados atuais disponíveis. Este resumo é preenchido automaticamente quando o Rullst/Benchmarks publica resultados que esta página consegue ler; até lá, ele não mostra números em vez de mostrar números antigos."</p> }
        } else {
            html! { <p class="benchmark-pending">"No current results are available. This summary fills in automatically when Rullst/Benchmarks publishes results this page can read; until then it shows no numbers rather than older ones."</p> }
        }
    });
    if pt {
        pt_top::page(cx, proof, summary)
    } else {
        en_top::page(cx, proof, summary)
    }
}

/// One card of "Included in Rullst": its label, title, what it does, what Loco and Topcoat
/// document when the card compares them, and a link (address, text) to its guide.
pub fn included_card(
    index: &str,
    title: Markup,
    body: Markup,
    elsewhere: Option<&str>,
    (href, link): (&str, &str),
) -> Markup {
    let elsewhere = elsewhere.map(|text| html! { <p class="included-elsewhere">{text}</p> });
    html! {
        <article class="included-card">
            <span class="included-index">{index}</span>" "
            <h3>{title}</h3>
            <p>{body}</p>
            {elsewhere}" "
            <a href={href}>{link}" "<span aria-hidden="true">"↗"</span></a>
        </article>
    }
}

/// One deployed example of the Examples section.
pub struct Demo<'a> {
    /// The card's class, which sets its colours (`demo-showcase`, …).
    pub class: &'a str,
    pub art: Markup,
    pub caption: &'a str,
    pub kicker: &'a str,
    pub title: &'a str,
    pub text: &'a str,
    pub href: &'a str,
    pub button: &'a str,
}

/// An example's card; `note` says that its container sleeps when idle.
fn demo(demo: &Demo, note: &str) -> Markup {
    html! {
        <article class={format!("demo-card {}", demo.class)}>
            <div class="demo-art" aria-hidden="true">
                <span class="demo-orbit"></span>" "
                {&demo.art}" "
                <span class="demo-art-caption">{demo.caption}</span>
            </div>
            <div class="demo-content">
                <span class="demo-kicker">{demo.kicker}</span>
                <h3>{demo.title}</h3>
                <p>{demo.text}</p>
                <p class="demo-host-note">{note}</p>
                <a class="button button-secondary" href={demo.href} data-demo-link>{demo.button}" "<span aria-hidden="true">"↗"</span></a>
            </div>
        </article>
    }
}

/// An English example card.
pub fn demo_card(card: &Demo) -> Markup {
    demo(
        card,
        "Runs in an Azure container that sleeps when idle, so the first load can take about 20–30 seconds.",
    )
}

/// A Portuguese example card.
pub fn demo_card_pt(card: &Demo) -> Markup {
    demo(
        card,
        "Roda em um contêiner do Azure que hiberna quando ocioso, então o primeiro carregamento pode levar de 20 a 30 segundos.",
    )
}
