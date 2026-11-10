//! `html!` attribute names written as Alpine.js and htmx write them, and
//! unquoted text (13.0.0-alpha.5), expanded by the real macro: spacing is
//! rebuilt from the source positions of the tokens, which only a compiled
//! expansion has.
#![allow(clippy::unwrap_used)]

extern crate rullst_core as rullst;
use rullst::html;

#[test]
fn alpine_and_htmx_attribute_names_are_written_as_declared() {
    let page = html! {
        <div x-data="{ open: false }" @click.outside="open = false" :class="{ 'open': open }">
            <button x-on:click.prevent="open = !open" x-bind:disabled="busy">"Menu"</button>
            <form hx-post="/save" hx-on::after-request="this.reset()" @submit.prevent.stop="go()">
                <input x-model.number="age" x-transition:enter.duration.500ms required :value="age" />
            </form>
            <svg><use xlink:href="#icon"></use></svg>
        </div>
    };
    assert_eq!(
        page,
        "<div x-data=\"{ open: false }\" @click.outside=\"open = false\" \
         :class=\"{ 'open': open }\">\
         <button x-on:click.prevent=\"open = !open\" x-bind:disabled=\"busy\">Menu</button>\
         <form hx-post=\"/save\" hx-on::after-request=\"this.reset()\" \
         @submit.prevent.stop=\"go()\">\
         <input x-model.number=\"age\" x-transition:enter.duration.500ms required \
         :value=\"age\" /></form>\
         <svg><use xlink:href=\"#icon\"></use></svg></div>"
    );
}

#[test]
fn dynamic_values_of_names_with_colons_are_escaped_and_url_checked() {
    let label = "a\"b<c>";
    let link = "javascript:alert(1)";
    let page = html! { <svg aria-label:x={label}><use xlink:href={link} /></svg> };
    assert_eq!(
        page,
        "<svg aria-label:x=\"a&quot;b&lt;c&gt;\"><use xlink:href=\"#\"></use></svg>"
    );
}

#[test]
fn unquoted_text_keeps_words_punctuation_and_single_spaces() {
    let name = "Ana";
    let count = 3;
    let page = html! {
        <h1>Hello, world!</h1>
        <p>Welcome back, {name}. You have {count} new messages (and 2 drafts).</p>
        <p>
            At 12:30   on day 5 - no, 1.5 days later;
            keep calm: 100% done & ready?
        </p>
        <p>Inbox ({count})</p>
        <p>Read <a href="/terms">the terms</a> before signing up.</p>
        <p>{name}'s <b>account</b>!</p>
    };
    assert_eq!(
        page,
        "<h1>Hello, world!</h1>\
         <p>Welcome back, Ana. You have 3 new messages (and 2 drafts).</p>\
         <p>At 12:30 on day 5 - no, 1.5 days later; keep calm: 100% done & ready?</p>\
         <p>Inbox (3)</p>\
         <p>Read <a href=\"/terms\">the terms</a> before signing up.</p>\
         <p>Ana's <b>account</b>!</p>"
    );
}

#[test]
fn whitespace_at_the_edges_of_content_is_dropped() {
    let done = true;
    let page = html! {
        <ul>
            <li>
                Write tests
            </li>
            <li> Ship it </li>
        </ul>
        @if done { All done } @else { Keep going }
        <p>
            Status:
            @if done { finished } @else { open }
            today
        </p>
    };
    assert_eq!(
        page,
        "<ul><li>Write tests</li><li>Ship it</li></ul>All done\
         <p>Status: finished today</p>"
    );
}

#[test]
fn quoted_literals_and_nodes_without_text_keep_their_output() {
    let name = "<Ana>";
    // Whitespace between two nodes that are not unquoted text is still
    // ignored, so these keep the output of earlier releases.
    let page = html! {
        <p>"Hello, " {name} "!"</p>
        <span>"a"</span> <span>"b"</span>
        <pre>"  exact   spaces  "</pre>
    };
    assert_eq!(
        page,
        "<p>Hello, &lt;Ana&gt;!</p><span>a</span><span>b</span><pre>  exact   spaces  </pre>"
    );
    // Unquoted text next to a string literal is separated as written.
    let mixed = html! { <p>Say "hello" now, ("quoted")</p> };
    assert_eq!(mixed, "<p>Say hello now, (quoted)</p>");
}

#[test]
fn keywords_and_non_ascii_words_are_text() {
    let page = html! {
        <p>Olá, São Paulo! if you match for else, true or false.</p>
        <p>Something else</p>
    };
    assert_eq!(
        page,
        "<p>Olá, São Paulo! if you match for else, true or false.</p><p>Something else</p>"
    );
}

#[test]
fn entities_written_unquoted_stay_character_references() {
    let page = html! { <p>Tom &amp; Jerry &copy; 2026 &lt;3</p> };
    assert_eq!(page, "<p>Tom &amp; Jerry &copy; 2026 &lt;3</p>");
}

/// The example of tutorial 06 ("Text and attribute names").
#[test]
fn the_tutorial_menu_renders_as_documented() {
    let count = 2;
    let menu = html! {
        <div x-data="{ open: false }" @click.outside="open = false">
            <button @click="open = !open" :aria-expanded="open">Menu ({count})</button>
            <form hx-post="/notes" hx-on::after-request="this.reset()">
                Write a note, then press <kbd>Enter</kbd>.
            </form>
        </div>
    };
    assert_eq!(
        menu,
        "<div x-data=\"{ open: false }\" @click.outside=\"open = false\">\
         <button @click=\"open = !open\" :aria-expanded=\"open\">Menu (2)</button>\
         <form hx-post=\"/notes\" hx-on::after-request=\"this.reset()\">\
         Write a note, then press <kbd>Enter</kbd>.</form></div>"
    );
}

#[test]
fn spacing_follows_the_source_around_groups_blocks_and_comments() {
    let (a, b, c) = (1, 2, 3);
    let page = html! {
        <p>{a}{b} {c}</p>
        <p>[ draft ] (a, b) 12:30, 1.5% & 3rd; a-b/c?</p>
        <p>one /* two */ three, one<!-- two -->three</p>
        <p>"a" {b} "c" then "quoted" text</p>
    };
    assert_eq!(
        page,
        "<p>123</p>\
         <p>[draft] (a, b) 12:30, 1.5% & 3rd; a-b/c?</p>\
         <p>one three, onethree</p>\
         <p>a2c then quoted text</p>"
    );
}

#[test]
fn a_colon_after_a_space_starts_the_next_attribute() {
    let page = html! {
        <input required :value="v" disabled:x="y" />
        <p :type="t" @click.self="s"></p>
    };
    assert_eq!(
        page,
        "<input required :value=\"v\" disabled:x=\"y\" /><p :type=\"t\" @click.self=\"s\"></p>"
    );
}
