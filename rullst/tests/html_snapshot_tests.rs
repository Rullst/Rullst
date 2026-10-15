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
