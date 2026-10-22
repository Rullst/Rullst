use rullst::html;

#[test]
#[cfg_attr(miri, ignore)]
fn test_html_macro_snapshot_simple() {
    let result = html! {
        <div class="test-container" id="main">
            <h1>"Rullst Snapshot Test"</h1>
            <p>"This ensures the HTML macro output remains exactly the same."</p>
        </div>
    };

    // We use insta to verify the generated HTML
    insta::assert_snapshot!(result, @"<div class=\"test-container\" id=\"main\"><h1>Rullst Snapshot Test</h1><p>This ensures the HTML macro output remains exactly the same.</p></div>");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_html_macro_snapshot_dynamic() {
    let user_name = "Alice";
    let is_admin = true;
    let items = ["Rust", "Security", "Performance"];

    let result = html! {
        <div class="dashboard">
            <header>
                <h2>"Welcome, "{user_name}</h2>
                {
                    is_admin.then(|| html! { <span class="badge">"Admin"</span> })
                }
            </header>
            <ul class="features">
                {
                    items.iter().map(|item| html! {
                        <li>{item}</li>
                    }).collect::<rullst::html::Markup>()
                }
            </ul>
        </div>
    };

    // Nested `html!` output is inserted as HTML; interpolated text is escaped.
    insta::assert_snapshot!(result, @"<div class=\"dashboard\"><header><h2>Welcome, Alice</h2><span class=\"badge\">Admin</span></header><ul class=\"features\"><li>Rust</li><li>Security</li><li>Performance</li></ul></div>");
}

#[test]
#[cfg_attr(miri, ignore)]
fn html_macro_accepts_and_strips_source_comments() {
    let result = html! {
        <!-- Page-level implementation note -->
        <main>
            <!-- Comments do not become client-visible output -->
            <p>"Audited content"</p>
        </main>
        <!-- Trailing implementation note -->
    };

    assert_eq!(result, "<main><p>Audited content</p></main>");
}

#[test]
#[cfg_attr(miri, ignore)]
fn html_macro_renders_a_doctype_keyword_elements_and_nested_views() {
    let title = "<Notes>";
    let icon = html! { <svg><use href="#note" /></svg> };
    let trusted = rullst::html::RawHtml::new("<hr>");
    let page = html! {
        <!DOCTYPE html>
        <html><body><h1>{title}</h1>{icon}{trusted}</body></html>
    };

    assert_eq!(
        page,
        "<!DOCTYPE html><html><body><h1>&lt;Notes&gt;</h1><svg><use href=\"#note\"></use></svg><hr></body></html>"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn html_macro_renders_bare_and_conditional_attributes_and_control_blocks() {
    // The `html!` documentation example in rullst-macros.
    let user: Option<&str> = Some("Ana");
    let items: Vec<&str> = "a b".split(' ').collect();
    let page = html! {
        @if let Some(name) = user { <p>"Hi " {name}</p> } @else { <a href="/login">"Sign in"</a> }
        <ul>
            @for (index, item) in items.iter().enumerate() {
                <li aria-current?={(index == 0).then_some("true")}>{item}</li>
            }
        </ul>
        <button disabled?={items.is_empty()}>"Export"</button>
        @match items.len() {
            0 => { <p>"No items"</p> }
            n if n > 9 => { <p>"Many items"</p> }
            n => { <p>{n} " items"</p> }
        }
    };
    assert_eq!(
        page,
        "<p>Hi Ana</p><ul><li aria-current=\"true\">a</li><li>b</li></ul>\
         <button>Export</button><p>2 items</p>"
    );

    let user: Option<&str> = None;
    let missing: Option<String> = None;
    let page = html! {
        @if let Some(name) = user { <p>{name}</p> } @else { <a href="/login" title?={missing}>"Sign in"</a> }
        <input name="q" required autofocus?={user.is_none()} value?={Some("<x>")} />
    };
    assert_eq!(
        page,
        "<a href=\"/login\">Sign in</a><input name=\"q\" required autofocus value=\"&lt;x&gt;\" />"
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn html_match_blocks_escape_bindings_and_keep_nested_markup() {
    enum Status<'a> {
        Draft,
        Published { title: &'a str, views: u32 },
        Archived(&'a str),
    }
    let badge = html! { <span class="badge">"new"</span> };
    let render = |status: &Status| {
        html! {
            <li>
                @match status {
                    Status::Draft => { <em>"Draft"</em> }
                    Status::Published { title, views } if *views == 0 => { {title} {&badge} }
                    Status::Published { title, views } => { {title} " (" {views} ")" }
                    Status::Archived(reason) => { <s title={reason}>"Archived"</s> }
                }
            </li>
        }
    };
    assert_eq!(render(&Status::Draft), "<li><em>Draft</em></li>");
    assert_eq!(
        render(&Status::Published {
            title: "<b>Hi</b>",
            views: 0
        }),
        "<li>&lt;b&gt;Hi&lt;/b&gt;<span class=\"badge\">new</span></li>"
    );
    assert_eq!(
        render(&Status::Published {
            title: "A & B",
            views: 7
        }),
        "<li>A &amp; B (7)</li>"
    );
    assert_eq!(
        render(&Status::Archived("\"old\"")),
        "<li><s title=\"&quot;old&quot;\">Archived</s></li>"
    );
}
