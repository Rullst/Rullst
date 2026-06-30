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
                    if is_admin {
                        html! { <span class="badge">"Admin"</span> }
                    } else {
                        String::new()
                    }
                }
            </header>
            <ul class="features">
                {
                    items.iter().map(|item| html! {
                        <li>{item}</li>
                    }).collect::<String>()
                }
            </ul>
        </div>
    };

    insta::assert_snapshot!(result, @"<div class=\"dashboard\"><header><h2>Welcome, Alice</h2>&lt;span class=&quot;badge&quot;&gt;Admin&lt;/span&gt;</header><ul class=\"features\">&lt;li&gt;Rust&lt;/li&gt;&lt;li&gt;Security&lt;/li&gt;&lt;li&gt;Performance&lt;/li&gt;</ul></div>");
}
