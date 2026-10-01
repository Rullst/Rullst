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
        // `Live::mount` supplies the `hx-ext="ws"` wrapper; the root needs a
        // stable id so HTMX can swap the re-rendered markup it receives.
        html! {
            <div id="live-counter-component" class="live-counter">
                <h2 class="live-counter-title">"Rullst Live (Server-Driven UI)"</h2>

                <div class="live-counter-value">
                    {self.count}
                </div>

                <form ws-send="true" class="live-counter-actions">
                    <button
                        type="submit"
                        name="rullst_event"
                        value="decrement"
                        aria-label="Decrease counter"
                        class="counter-btn counter-btn-dec"
                    >
                        "- Decrease"
                    </button>
                    <button
                        type="submit"
                        name="rullst_event"
                        value="increment"
                        aria-label="Increase counter"
                        class="counter-btn counter-btn-inc"
                    >
                        "+ Increase"
                    </button>
                </form>

                <p class="live-counter-note">
                    "All state is maintained on the server; the same-origin HTMX WebSocket extension sends events and applies the re-rendered markup."
                </p>
            </div>
        }
    }
}
