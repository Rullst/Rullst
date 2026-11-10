use rullst_macros::html;

fn main() {
    let state = "{ open: false }";
    let _ = html! {
        <div x-data={state} @click="open = !open"></div>
    };
}
