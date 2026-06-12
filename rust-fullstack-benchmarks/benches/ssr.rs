use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dioxus::prelude::*;
use leptos::view;
use rullst::html;
use tera::{Context, Tera};

// -- Dioxus Component --
fn dioxus_list() -> Element {
    let items = vec!["Rust", "Go", "Python"];
    rsx! {
        div { class: "user-profile",
            h2 { "Hello, Alice!" }
            ul {
                for item in items {
                    li { "{item}" }
                }
            }
        }
    }
}

// -- Leptos Component --
#[leptos::component]
fn LeptosList() -> impl leptos::IntoView {
    let items = vec!["Rust", "Go", "Python"];
    view! {
        <div class="user-profile">
            <h2>"Hello, Alice!"</h2>
            <ul>
                {items.into_iter()
                    .map(|item| view! { <li>{item}</li> })
                    .collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

fn bench_ssr(c: &mut Criterion) {
    let mut group = c.benchmark_group("SSR Rendering");

    // 1. Rullst `html!` macro (Compiled directly to strings)
    group.bench_function("Rullst (html! macro)", |b| {
        let name = "Alice";
        let items = ["Rust", "Go", "Python"];
        b.iter(|| {
            let html = html! {
                <div class="user-profile">
                    <h2>"Hello, "{name}"!"</h2>
                    <ul>
                        { items.iter().map(|item| html! { <li>{item}</li> }).fold(String::with_capacity(256), |mut acc, s| { acc.push_str(&s); acc }) }
                    </ul>
                </div>
            };
            black_box(html);
        });
    });

    // 2. Dioxus SSR (Virtual DOM rendering to string)
    group.bench_function("Dioxus (Virtual DOM to String)", |b| {
        b.iter(|| {
            let mut dom = VirtualDom::new(dioxus_list);
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            black_box(html);
        });
    });

    // 3. Leptos SSR (String templates)
    group.bench_function("Leptos (View macro to String)", |b| {
        b.iter(|| {
            let html = leptos::ssr::render_to_string(|| view! { <LeptosList/> }).to_string();
            black_box(html);
        });
    });

    // 4. Tera templates (Standard Engine like in Loco or traditional frameworks)
    let mut tera = Tera::default();
    tera.add_raw_template(
        "profile.html",
        r#"
        <div class="user-profile">
            <h2>Hello, {{ name }}!</h2>
            <ul>
                {% for item in items %}
                <li>{{ item }}</li>
                {% endfor %}
            </ul>
        </div>
        "#,
    )
    .unwrap();

    group.bench_function("Tera Template Engine", |b| {
        b.iter(|| {
            let mut context = Context::new();
            context.insert("name", "Alice");
            context.insert("items", &vec!["Rust", "Go", "Python"]);
            let html = tera.render("profile.html", &context).unwrap();
            black_box(html);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_ssr);
criterion_main!(benches);
