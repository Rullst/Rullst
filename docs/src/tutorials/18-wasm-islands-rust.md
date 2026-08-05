# Tutorial 18: Wasm Islands in Pure Rust 🧩

Write high-performance interactive client-side components in pure Rust compiled to WebAssembly.

---

## 🛠️ Step 1: Scaffold a Wasm Island

```bash
cargo rullst make:island InteractiveChart
```

This creates `src/islands/interactive_chart.rs`.

---

## 💻 Step 2: Write Client Component Code

```rust
use rullst::client_component;
use web_sys::window;

#[client_component]
pub fn render_chart() {
    let window = window().expect("Global window missing");
    let document = window.document().expect("Document missing");
    
    if let Some(element) = document.get_element_by_id("chart-container") {
        element.set_inner_html("<p class='text-emerald-400 font-bold'>Wasm Island Mounted!</p>");
    }
}
```

---

## 🚀 Step 3: Build WebAssembly Binaries

```bash
cargo rullst build:client
```

This invokes `wasm-pack` to compile your client components into optimized WebAssembly assets.

---

## 💡 Key Takeaways
- Use **Wasm Islands** for heavy client-side interactivity (canvas graphs, rich text editors, cryptography in the browser).
- Avoid JS bundlers and Webpack setup.
