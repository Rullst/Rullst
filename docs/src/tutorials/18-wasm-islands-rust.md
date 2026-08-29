# Tutorial 18: Wasm Island foundation

Rullst can generate a dual-target Island function: native builds emit a host
element with serialized props, while `wasm32-unknown-unknown` builds export a
hydration function.

## Generate the component

```bash
cargo rullst make:island InteractiveChart
```

The generated `src/islands/interactive_chart.rs` uses the supported macro:

```rust
use rullst::island;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractiveChartProps {
    pub initial_value: i32,
}

#[island]
pub fn interactive_chart(props: InteractiveChartProps) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        format!(
            "<button type=\"button\">Count: {}</button>",
            props.initial_value
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut value = props.initial_value;
        element.set_text_content(Some(&format!("Count: {value}")));

        let button = element.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            value = value.saturating_add(1);
            button.set_text_content(Some(&format!("Count: {value}")));
        });
        if element
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .is_ok()
        {
            closure.forget();
        }

        String::new()
    }
}
```

The `element` binding in the Wasm block is supplied by `#[island]`.

## Build and load the artifact

```bash
cargo rullst build:client
```

The command parses `Cargo.toml`, adds `cdylib` to the existing `[lib]`
`crate-type` array when needed, installs/checks the Wasm target and
`wasm-bindgen-cli`, builds the library, locates the artifact using `lib.name` or
`package.name`, writes bindings under `static/`, and generates a hydration
orchestrator. Review these manifest, network, and toolchain side effects in CI
and pin the required tools for reproducible releases. Load the generated ES
module from the page as instructed by the command output.

This is a useful foundation, not a complete frontend framework: routing,
application state, accessibility, CSP-compatible asset delivery, cache busting,
error reporting and browser E2E remain application/release work.
