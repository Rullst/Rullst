//! The six pages of the root site, in both languages.

mod compare;
mod landing;
pub mod made_with;
mod privacy;

use rullst::html;
use rullst::html::Markup;

use crate::assets::Assets;
use crate::benchmarks::{self, Results};
use crate::footprint::Weights;

/// The `cargo-rullst` version the install commands pin: the current release in
/// `cargo-rullst/Cargo.toml` (`validate-site.py` checks it).
pub const INSTALL_VERSION: &str = "13.0.0-alpha.6";

/// The `rullst` crate these pages are made with (build.rs reads it from `site/Cargo.lock`), and
/// the `cargo-rullst` version the deployment installs to weigh the published landing pages.
pub const RULLST_VERSION: &str = env!("RULLST_VERSION");

/// A page's language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
}

impl Lang {
    pub fn is_pt(self) -> bool {
        self == Lang::PtBr
    }
}

/// What every page reads: the asset references, and the page weights that passed their checks.
pub struct Context<'a> {
    pub assets: &'a Assets,
    pub weights: &'a Weights,
}

/// The comparison page's text while no current results can be shown.
fn results_pending(lang: Lang) -> Markup {
    if lang.is_pt() {
        html! { <p class="benchmark-pending">"Não há resultados atuais disponíveis. Esta seção se preenche automaticamente quando o Rullst/Benchmarks publicar resultados que esta página consiga ler; até lá, ela não mostra números, em vez de mostrar números antigos."</p> }
    } else {
        html! { <p class="benchmark-pending">"No current results are available. This section fills in automatically when Rullst/Benchmarks publishes results this page can read; until then it shows no numbers rather than older ones."</p> }
    }
}

/// Every page, by published path, in a fixed order.
pub fn render(
    assets: &Assets,
    results: Option<&Results>,
    weights: &Weights,
) -> Vec<(&'static str, Markup)> {
    let cx = Context { assets, weights };
    let compared = |lang| {
        results.map_or_else(
            || results_pending(lang),
            |results| benchmarks::compare::render(results, lang),
        )
    };
    let landing_blocks = results.and_then(|results| {
        let blocks = (
            benchmarks::landing::render(results, Lang::En),
            benchmarks::landing::render(results, Lang::PtBr),
        );
        if blocks.0.is_none() || blocks.1.is_none() {
            eprintln!(
                "warning: landing benchmark summary not rendered (the results have no Rullst row)"
            );
        }
        Some(blocks).filter(|(en, pt)| en.is_some() && pt.is_some())
    });
    let (en_blocks, pt_blocks) = landing_blocks.unwrap_or((None, None));
    vec![
        ("index.html", landing::page(&cx, Lang::En, en_blocks)),
        ("privacy.html", privacy::en::page(&cx)),
        (
            "pt-br/index.html",
            landing::page(&cx, Lang::PtBr, pt_blocks),
        ),
        ("pt-br/privacy.html", privacy::pt::page(&cx)),
        (
            "compare/index.html",
            compare::en::page(&cx, compared(Lang::En)),
        ),
        (
            "pt-br/comparar/index.html",
            compare::pt::page(&cx, compared(Lang::PtBr)),
        ),
    ]
}
