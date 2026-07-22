#![allow(non_snake_case)]
use rullst::{html, island};

#[island]
#[allow(non_snake_case)]
pub fn InteractiveCounter(initial_count: i32) -> String {
    // Server-side HTML (used for SSR)
    #[cfg(not(target_arch = "wasm32"))]
    {
        html! {
            <div style="background: #1e293b; padding: 2.5rem; border-radius: 1rem; text-align: center; max-width: 420px; margin: 3rem auto; color: white; border: 1px solid #334155; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3);">
                <h2 style="margin-top: 0; font-size: 1.5rem; color: #818cf8;">"Rullst Wasm Island 🏝️"</h2>
                <p style="color: #94a3b8; font-size: 0.95rem; margin-bottom: 1.5rem;">"Compilado nativamente para WebAssembly. Hidratação 100% transparente!"</p>

                <div style="font-size: 4rem; font-weight: 800; margin: 1.5rem 0; color: #fff; font-family: monospace;">
                    <span id="counter-value">{initial_count}</span>
                </div>

                <div style="display: flex; gap: 1rem; justify-content: center;">
                    <button id="counter-dec" aria-label="Decrease counter" style="padding: 0.75rem 1.75rem; background: #ef4444; color: white; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: bold; font-size: 1.1rem; transition: background 0.2s;">
                        "- Diminuir"
                    </button>
                    <button id="counter-inc" aria-label="Increase counter" style="padding: 0.75rem 1.75rem; background: #10b981; color: white; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: bold; font-size: 1.1rem; transition: background 0.2s;">
                        "+ Aumentar"
                    </button>
                </div>
            </div>
        }
    }

    // Client-side Hydration
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let count_state = std::rc::Rc::new(std::cell::Cell::new(initial_count));

        // Find buttons and counter span inside the component root (element)
        let btn_dec = element.query_selector("#counter-dec").unwrap().unwrap();
        let btn_inc = element.query_selector("#counter-inc").unwrap().unwrap();
        let val_span = element.query_selector("#counter-value").unwrap().unwrap();

        // Setup increment callback
        {
            let count = count_state.clone();
            let val_span = val_span.clone();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                let new_count = count.get() + 1;
                count.set(new_count);
                val_span.set_text_content(Some(&new_count.to_string()));
            }) as Box<dyn FnMut()>);

            btn_inc
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        // Setup decrement callback
        {
            let count = count_state.clone();
            let val_span = val_span.clone();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                let new_count = count.get() - 1;
                count.set(new_count);
                val_span.set_text_content(Some(&new_count.to_string()));
            }) as Box<dyn FnMut()>);

            btn_dec
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
    }
}
