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
