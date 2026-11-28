//! Static, bare, dynamic and conditional attributes, and the URL check.

use std::borrow::Cow;

use rullst::html;
use rullst::html::{Markup, RawHtml};

use super::Case;

const SCRIPT: &str = "javascript:alert(1)";
const DATA: &str = "data:text/html,<b>x</b>";

pub fn cases() -> Vec<Case> {
    let mut cases = fixed();
    cases.extend(dynamic());
    cases.extend(urls());
    cases.extend(conditional());
    cases
}

fn fixed() -> Vec<Case> {
    vec![
        (
            "attributes/static",
            html! {
                <meta name="htmx-config" content="{\"includeIndicatorStyles\":false}" title="a &amp; <b> 'q'" />
            },
        ),
        (
            "attributes/bare",
            html! { <input required disabled hx-boost data-x type="checkbox" checked /> },
        ),
        (
            "attributes/alpine-and-htmx-names",
            html! {
                <div x-data="{ open: false }" @click.outside="open = false" :class="{ 'open': open }">
                    <button x-on:click.prevent="open = !open" x-bind:disabled="busy">"Menu"</button>
                    <form hx-post="/save" hx-on::after-request="this.reset()" @submit.prevent.stop="go()">
                        <input x-model.number="age" x-transition:enter.duration.500ms required :value="age" />
                    </form>
                </div>
            },
        ),
        (
            "attributes/colon-after-space",
            html! { <input required :value="v" disabled:x="y" /><p :type="t" @click.self="s"></p> },
        ),
        (
            "attributes/static-handlers",
            html! { <button onclick="go()" hx-on-click="go()" onclick>"Go"</button> },
        ),
    ]
}

#[allow(deprecated)]
fn dynamic() -> Vec<Case> {
    let text = "a\"b<c>&d'e";
    let owned = String::from("<owned & \"quoted\">");
    let cow: Cow<'_, str> = Cow::Owned("c<o>w".to_string());
    let markup = html! { <b title="t">"x & y"</b> };
    let some: Option<&str> = Some("<some>");
    let none: Option<&str> = None;
    vec![
        (
            "attributes/dynamic-strings",
            html! {
                <p title={text} data-owned={owned} data-ref={&owned} data-cow={cow} data-refref={&&text} data-empty={""}></p>
            },
        ),
        (
            "attributes/dynamic-numbers-and-bools",
            html! {
                <input value={42} min={-7_i64} max={u64::MAX} step={0.5} data-f={-0.0_f64} aria-busy={true} aria-hidden={false} />
            },
        ),
        (
            "attributes/dynamic-markup-is-escaped",
            html! { <p title={markup} data-raw={RawHtml("<i>raw</i>".to_string())}></p> },
        ),
        (
            "attributes/dynamic-options",
            html! { <p title={some} data-none={none} data-ref={&some} data-num={Some(5)}></p> },
        ),
        (
            "attributes/dynamic-expressions",
            html! {
                <p class={if text.is_empty() { "a" } else { "b" }} title={format!("{}-{}", 1, "<2>")} data-len={text.len()}></p>
            },
        ),
        (
            "attributes/order-of-every-kind",
            html! {
                <input type="text" required name={text} checked?={true} hidden?={false} title?={some} lang?={none} data-x="y" />
            },
        ),
    ]
}

#[allow(deprecated)]
fn urls() -> Vec<Case> {
    let script = SCRIPT;
    let data = DATA;
    let markup = Markup::from_trusted("javascript:x");
    vec![
        (
            "urls/every-url-attribute",
            html! {
                <form action={script} data={script} formaction={script}>
                    <a href={script} cite={script} xlink:href={script}>"a"</a>
                    <img src={script} longdesc={script} usemap={script} />
                    <video poster={script}></video>
                    <body background={script}></body>
                    <object codebase={script}></object>
                    <menuitem icon={script}></menuitem>
                    <html manifest={script}></html>
                </form>
            },
        ),
        (
            "urls/names-in-any-case",
            html! { <a HREF={script} Src={script}>"a"</a> },
        ),
        (
            "urls/other-attributes-keep-the-value",
            html! { <a title={script} data-href={script} hrefs={script}>"a"</a> },
        ),
        (
            "urls/data-on-media-only",
            html! {
                <div>
                    <img src={data} /><IMG src={data} /><image href={data}></image>
                    <audio src={data}></audio><video src={data} poster={data}></video>
                    <source src={data} /><track src={data} />
                    <a href={data}>"a"</a><iframe src={data}></iframe>
                </div>
            },
        ),
        (
            "urls/schemes",
            html! {
                <p>
                    <a href={"JavaScript:alert(1)"}>"1"</a>
                    <a href={"vbscript:x"}>"2"</a>
                    <a href={"  \u{1}\tjavascript:x"}>"3"</a>
                    <a href={"java\tscr\nipt:x"}>"4"</a>
                    <a href={"javascriptx:x"}>"5"</a>
                    <a href={"jav ascript:x"}>"6"</a>
                    <a href={"&#106;avascript:x"}>"7"</a>
                    <a href={"mailto:a@b.c"}>"8"</a>
                    <a href={"/path?a=1&b=\"2\""}>"9"</a>
                    <a href={"https://example.com/a?b=<c>"}>"10"</a>
                    <a href={"#top"}>"11"</a>
                    <a href={""}>"12"</a>
                    <a href={":x"}>"13"</a>
                    <a href={"data"}>"14"</a>
                </p>
            },
        ),
        (
            "urls/value-types",
            html! {
                <p>
                    <a href={5}>"n"</a>
                    <a href={markup}>"m"</a>
                    <a href={RawHtml("javascript:raw".to_string())}>"r"</a>
                    <a href={Some(SCRIPT)}>"s"</a>
                    <a href={None::<&str>}>"none"</a>
                    <a href={String::from("vbscript:y")}>"owned"</a>
                </p>
            },
        ),
    ]
}

fn conditional() -> Vec<Case> {
    let title: Option<String> = Some("<t> & \"q\"".to_string());
    let missing: Option<String> = None;
    let flag = true;
    vec![
        (
            "conditional/bools",
            html! { <input type="checkbox" checked?={flag} disabled?={!flag} required?={&&flag} /> },
        ),
        (
            "conditional/options",
            html! {
                <p title?={title} data-missing?={missing} data-ref?={&title} data-num?={Some(-3)} data-markup?={Some(html! { <b>"m"</b> })}></p>
            },
        ),
        (
            "conditional/urls",
            html! {
                <p>
                    <a href?={Some(SCRIPT)} cite?={Some("/path?a=1")}>"a"</a>
                    <img src?={Some(DATA)} />
                    <a href?={Some(DATA)}>"b"</a>
                    <a href?={None::<&str>}>"c"</a>
                    <a title?={Some(SCRIPT)}>"d"</a>
                </p>
            },
        ),
    ]
}
