use rullst::html;
use rullst::live_component;

/// Our Rullst Live component. All state lives on and is managed by the server!
#[live_component]
#[derive(Default)]
pub struct CounterComponent {
    pub count: i32,
}

#[live_component]
impl CounterComponent {
    pub fn mount(&mut self) {
        // Initialize state. You could even fetch things from the DB here using rullst-orm!
        self.count = 0;
    }

    #[live_event]
    pub fn increment(&mut self) {
        self.count += 1;
    }

    #[live_event]
    pub fn decrement(&mut self) {
        self.count -= 1;
    }

    pub fn render(&self) -> String {
        // Renderizamos a interface.
        // O hx-ext="ws" no root será fornecido pelo Live::mount wrapper,
        // mas devemos colocar um ID no container principal para que o HTMX saiba o que substituir via WebSocket DOM Swap.
        html! {
            <div id="live-counter-component" style="background: #1e293b; padding: 2rem; border-radius: 12px; text-align: center; max-width: 400px; margin: 3rem auto; color: white; box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.1);">
                <h2 style="margin-top: 0; font-size: 1.5rem; color: #38bdf8;">"Rullst Live (Server-Driven UI)"</h2>

                <div style="font-size: 4rem; font-weight: 800; margin: 2rem 0; color: #fff;">
                    {self.count}
                </div>

                <form ws-send="true" style="display: flex; gap: 1rem; justify-content: center; margin: 0;">
                    <button
                        type="submit"
                        name="rullst_event"
                        value="decrement"
                        aria-label="Decrease counter"
                        style="padding: 0.75rem 1.5rem; background: #e11d48; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; transition: all 0.2s;"
                    >
                        "- Decrease"
                    </button>
                    <button
                        type="submit"
                        name="rullst_event"
                        value="increment"
                        aria-label="Increase counter"
                        style="padding: 0.75rem 1.5rem; background: #10b981; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; transition: all 0.2s;"
                    >
                        "+ Increase"
                    </button>
                </form>

                <p style="font-size: 0.85rem; color: #94a3b8; margin-top: 1.5rem;">
                    "✨ Rust Magic: Zero JS files created. All state is maintained on the server and re-renders are sent via WebSockets by Rullst!"
                </p>
            </div>
        }
    }
}
