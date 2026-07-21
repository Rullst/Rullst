use rullst::{live_component, live_event};
use rullst::html;

/// Nosso componente Rullst Live. Todo o estado vive e é operado pelo servidor!
#[live_component]
#[derive(Default)]
pub struct CounterComponent {
    pub count: i32,
}

#[live_component]
impl CounterComponent {
    pub fn mount(&mut self) {
        // Inicializa o estado. Você poderia até buscar coisas do DB aqui usando o rullst-orm!
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

                <div style="display: flex; gap: 1rem; justify-content: center;">

                    <button
                        hx-vals=r#"{"rullst_event": "decrement"}"#
                        aria-label="Decrease counter"
                        style="padding: 0.75rem 1.5rem; background: #e11d48; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; transition: all 0.2s;"
                    >
                        "- Diminuir"
                    </button>
                    <button
                        hx-vals=r#"{"rullst_event": "increment"}"#
                        aria-label="Increase counter"
                        style="padding: 0.75rem 1.5rem; background: #10b981; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: bold; transition: all 0.2s;"
                    >
                        "+ Aumentar"
                    </button>
                </div>

                <p style="font-size: 0.85rem; color: #94a3b8; margin-top: 1.5rem;">
                    "✨ Mágica do Rust: Nenhum arquivo JS criado. Todo o estado é mantido no servidor e as re-renderizações são feitas e enviadas via WebSockets pelo Rullst!"
                </p>
            </div>
        }
    }
}
