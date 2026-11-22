//! Markup that several pages share: the head's links, the language switcher, the
//! navigation of the comparison and privacy pages, and a comparison topic.

use rullst::html;
use rullst::html::Markup;

use super::Lang;

/// The links a page's head ends with: its canonical URL, both languages, the icon and the
/// content-versioned stylesheet.
pub fn head_links(canonical: &str, en: &str, pt: &str, css: &str) -> Markup {
    html! {
        <link rel="canonical" href={canonical} />
        <link rel="alternate" hreflang="en" href={en} />
        <link rel="alternate" hreflang="pt-BR" href={pt} />
        <link rel="alternate" hreflang="x-default" href={en} />
        <link rel="icon" type="image/png" href="/Rullst/Rullst.png" />
        <link rel="stylesheet" href={css} />
    }
}

/// The flag language switcher, with the page's own language marked as current. The landings
/// separate the two links with a space; the other pages write them together.
pub fn lang_switch(lang: Lang, en: &str, pt: &str, spaced: bool) -> Markup {
    let current = |mine: Lang| (lang == mine).then_some("page");
    let group = if lang.is_pt() { "Idioma" } else { "Language" };
    let space = spaced.then_some(" ");
    html! {
        <div class="lang-switch" role="group" aria-label={group}><a href={en} hreflang="en" lang="en" aria-current?={current(Lang::En)}><img src="/assets/flags/us.svg" alt="" width="27" height="14" />"English"</a>{space}<a href={pt} hreflang="pt-BR" lang="pt-BR" aria-current?={current(Lang::PtBr)} aria-label="Português (Brasil)"><img src="/assets/flags/br.svg" alt="" width="20" height="14" />"Português"</a></div>
    }
}

/// The skip link and navigation of the comparison and privacy pages: the brand, the
/// language switcher and a link `back` to a section of the landing.
pub fn static_nav(lang: Lang, en: &str, pt: &str, back: &str) -> Markup {
    let pt_page = lang.is_pt();
    let (skip, label, home, back_text) = if pt_page {
        (
            "Pular para o conteúdo",
            "Navegação principal",
            "/pt-br/",
            "Voltar ao site ↗",
        )
    } else {
        (
            "Skip to content",
            "Primary navigation",
            "/",
            "Back to the website ↗",
        )
    };
    html! {
        <a class="skip-link" href="#main">{skip}</a>" "
        <nav class="site-nav" aria-label={label}>
            <div class="shell nav-inner">
                <a class="brand" href={home}>"rullst"<span class="brand-period">"."</span></a>
                {lang_switch(lang, en, pt, false)}
                <a href={back}>{back_text}</a>
            </div>
        </nav>
    }
}

/// One comparison topic: what Rullst, Loco and Topcoat each document about it.
pub fn topic(id: &str, title: &str, rullst: Markup, loco: Markup, topcoat: Markup) -> Markup {
    html! {
        <div class="compare-topic" id={id}>
            <h3>{title}</h3>
            <dl>
                <div><dt>"Rullst"</dt><dd>{rullst}</dd></div>
                <div><dt>"Loco"</dt><dd>{loco}</dd></div>
                <div><dt>"Topcoat"</dt><dd>{topcoat}</dd></div>
            </dl>
        </div>
    }
}
