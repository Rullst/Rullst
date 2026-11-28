//! Text, comments, the doctype and elements.

use rullst::html;

use super::Case;

pub fn cases() -> Vec<Case> {
    let name = "Ana";
    let count = 3;
    let done = true;
    let (a, b, c) = (1, 2, 3);
    vec![
        ("text/empty", html! {}),
        ("text/single-literal", html! { "plain" }),
        ("text/single-unquoted", html! { Hello there }),
        ("text/single-block", html! { {name} }),
        (
            "text/literal-is-written-as-is",
            html! { <p>"9 > 8 & 'x' \"q\" <b> &amp;"</p> },
        ),
        (
            "text/unquoted-sentence",
            html! { <p>Welcome back, {name}! You have {count} new messages (and 2 drafts).</p> },
        ),
        (
            "text/unquoted-edges",
            html! {
                <ul>
                    <li>
                        Write tests
                    </li>
                    <li> Ship it </li>
                </ul>
            },
        ),
        (
            "text/groups",
            html! { <p>[ draft ] (a, b) ({count}) [{name}] ( {name} )</p> },
        ),
        (
            "text/apostrophes",
            html! { <p>{name}'s turn, "don't stop" ({name}'s)</p> },
        ),
        (
            "text/numbers-and-punctuation",
            html! { <p>At 12:30 on day 5 - no, 1.5 days later; keep calm: 100% done & ready?</p> },
        ),
        (
            "text/number-literals",
            html! { <p>0x1F 1e3 2.50 7_000 1u8</p> },
        ),
        (
            "text/entities",
            html! { <p>Tom &amp; Jerry &copy; 2026 &lt;3</p> },
        ),
        (
            "text/keywords",
            html! { <p>if you match for else, true or false.</p> },
        ),
        ("text/non-ascii", html! { <p>Olá, São Paulo! Ça va?</p> }),
        (
            "text/pre-keeps-spaces",
            html! { <pre>"  exact   spaces\n\tand a tab  "</pre> },
        ),
        (
            "text/emoji-literal",
            html! { <span>"🏠 Control Center"</span><span>"フレームワーク"</span> },
        ),
        (
            "text/mixed-literals",
            html! { <p>Say "hello" now, ("quoted")</p> },
        ),
        (
            "text/blocks-and-literals",
            html! {
                <p>"Hello, " {name} "!"</p>
                <p>{a}{b} {c}</p>
                <p>"a" {b} "c" then "quoted" text</p>
            },
        ),
        (
            "text/comments-in-text",
            html! { <p>one /* two */ three, one<!-- two -->three</p> },
        ),
        (
            "text/control-edges",
            html! {
                <p>
                    Status:
                    @if done { finished } @else { open }
                    today
                </p>
                @if done { All done } @else { Keep going }
            },
        ),
        (
            "text/comments",
            html! {
                <!-- before -->
                <div><!-- inside -->"x"<!-- after x --></div>
                <!-- trailing -->
            },
        ),
        (
            "text/doctype-upper",
            html! { <!DOCTYPE html><html><body></body></html> },
        ),
        (
            "text/doctype-lower",
            html! { <!doctype HTML> <html lang="pt-BR"></html> },
        ),
        (
            "text/doctype-after-comment",
            html! { <!-- generated --><!DOCTYPE html><html></html> },
        ),
        (
            "elements/siblings",
            html! { <span>"a"</span> <span>"b"</span> "c" {name} <b></b> },
        ),
        (
            "elements/deep",
            html! {
                <html><head><title>"T"</title></head><body><main><section><div><p><b><i>"deep"</i></b></p></div></section></main></body></html>
            },
        ),
        (
            "elements/self-closing-non-void",
            html! { <div /><span class="x"/><use></use> },
        ),
        (
            "elements/void",
            html! {
                <area /><base /><br /><col /><embed /><hr /><img /><input /><link />
                <meta /><param /><source /><track /><wbr />
            },
        ),
        (
            "elements/void-written-open-close",
            html! { <br></br><hr class="x"></hr> },
        ),
        ("elements/void-with-children", html! { <br>"x"</br> }),
        (
            "elements/uppercase-tags",
            html! { <DIV><IMG src="a.png" /><Br /></DIV> },
        ),
        (
            "elements/svg",
            html! {
                <svg aria-hidden="true" viewBox="0 0 24 24" stroke-width="2">
                    <path stroke-linecap="round" d="M12 9v2m0 4h.01" />
                    <use href="#i"/>
                    <use xlink:href="#icon"></use>
                </svg>
            },
        ),
    ]
}
