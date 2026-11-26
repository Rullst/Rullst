//! "Made with Rullst": the landing's card of the maintainer's applications built with Rullst,
//! and the line every page's footer carries with the Rullst version these pages are made with.

use rullst::html;
use rullst::html::Markup;

use super::{Lang, RULLST_VERSION};

/// The source of these pages.
pub const SOURCE: &str = "https://github.com/Rullst/Rullst/tree/main/site";

/// One application: its name, address and one line about it in each language.
struct Application {
    name: &'static str,
    href: &'static str,
    /// The address on the Portuguese page, when the application has its own.
    href_pt: Option<&'static str>,
    en: &'static str,
    pt: &'static str,
}

/// The maintainer's live applications built with Rullst, each line from its own front page.
const APPLICATIONS: [Application; 5] = [
    Application {
        name: "Rullst Academy",
        href: "https://academy.rullst.win/",
        href_pt: Some("https://academy.rullst.win/pt-BR"),
        en: "Free lessons and learning paths in Rust and Rullst, from a first program in the terminal to a deployed web application.",
        pt: "Lições e trilhas gratuitas de Rust e Rullst, do primeiro programa no terminal a uma aplicação web publicada.",
    },
    Application {
        name: "Capoeira Vene",
        href: "https://capoeira.vene.win/",
        href_pt: None,
        en: "Capoeira classes in Fortaleza, Ceará, booked through WhatsApp or the site.",
        pt: "Aulas de capoeira em Fortaleza, Ceará, marcadas pelo WhatsApp ou pelo site.",
    },
    Application {
        name: "vene.win",
        href: "https://vene.win/",
        href_pt: None,
        en: "Tio Vené's private lessons in programming, game making and AI, with a calendar of technology competitions from 2026 to 2030.",
        pt: "Aulas particulares de programação, criação de jogos e inteligência artificial do Tio Vené, com um calendário de competições de tecnologia de 2026 a 2030.",
    },
    Application {
        name: "Rullst Benchmarks",
        href: "/Benchmarks/",
        href_pt: None,
        en: "The cross-framework benchmark's results website: static pages written by Rullst's html! macro from the published results.",
        pt: "O site de resultados do benchmark entre frameworks: páginas estáticas escritas pela macro html! do Rullst a partir dos resultados publicados.",
    },
    Application {
        name: "rullst.win",
        href: SOURCE,
        href_pt: None,
        en: "These pages, in English and Portuguese, written as static files by Rullst's html! macro; the link opens their source.",
        pt: "Estas páginas, em inglês e português, escritas como arquivos estáticos pela macro html! do Rullst; o link abre o código-fonte delas.",
    },
];

/// The landing's "Made with Rullst" card.
pub fn card(lang: Lang) -> Markup {
    let pt = lang.is_pt();
    let items: Markup = APPLICATIONS
        .iter()
        .map(|app| {
            let href = if pt {
                app.href_pt.unwrap_or(app.href)
            } else {
                app.href
            };
            let line = if pt { app.pt } else { app.en };
            html! { <li><a href={href}>{app.name}" ↗"</a><span>{line}</span></li> }
        })
        .collect();
    if pt {
        html! {
            <article class="made-with" id="made-with" aria-labelledby="made-with-heading">
                <span class="made-with-index">"NO AR · FEITO COM RULLST"</span>
                <h2 id="made-with-heading">"Aplicações no ar, feitas com Rullst"</h2>
                <p>"As aplicações do mantenedor que estão no ar e foram construídas com Rullst. Academy, capoeira e vene.win rodam em produção numa máquina Oracle Ampere A1 Arm; os dois sites de páginas estáticas ficam no GitHub Pages."</p>
                <ul>{items}</ul>
            </article>
        }
    } else {
        html! {
            <article class="made-with" id="made-with" aria-labelledby="made-with-heading">
                <span class="made-with-index">"LIVE · MADE WITH RULLST"</span>
                <h2 id="made-with-heading">"Live applications made with Rullst"</h2>
                <p>"The maintainer's applications that are live and built with Rullst. Academy, capoeira and vene.win run in production on one Oracle Ampere A1 Arm machine; the two static websites are on GitHub Pages."</p>
                <ul>{items}</ul>
            </article>
        }
    }
}

/// The footer line: the Rullst version these pages are made with, and `extra` after it.
pub fn line(lang: Lang, extra: Option<Markup>) -> Markup {
    if lang.is_pt() {
        html! { <p class="made-with-line">"Feito com "<a href={SOURCE}>"Rullst "{RULLST_VERSION}</a>": estas páginas são escritas pela macro "<code>"html!"</code>" do Rullst."{extra.map(|extra| html! { " "{extra} })}</p> }
    } else {
        html! { <p class="made-with-line">"Made with "<a href={SOURCE}>"Rullst "{RULLST_VERSION}</a>": these pages are written by Rullst's "<code>"html!"</code>" macro."{extra.map(|extra| html! { " "{extra} })}</p> }
    }
}

/// The short form for the footers of the comparison and privacy pages.
pub fn short(lang: Lang) -> Markup {
    if lang.is_pt() {
        html! { <span class="made-with-line">"Feito com "<a href={SOURCE}>"Rullst "{RULLST_VERSION}</a></span> }
    } else {
        html! { <span class="made-with-line">"Made with "<a href={SOURCE}>"Rullst "{RULLST_VERSION}</a></span> }
    }
}
